use crate::api::errors::APIError;
use crate::api::ok_json;
use crate::models::{File, GalleryItem};
use crate::schema::files::dsl as FilesDSL;
use crate::schema::gallery_files::dsl as GalleryFilesDSL;
use crate::schema::gallery_items::dsl as GalleryItemsDSL;
use crate::state::{self, AppState};
use actix_validated_forms::form::ValidatedForm;
use actix_validated_forms::multipart::{MultipartFile, ValidatedMultipartForm};
use actix_validated_forms::tempfile::NamedTempFile;
use actix_web::web::{Data, Path};
use actix_web::{web, HttpResponse};
use diesel::prelude::*;
use diesel::Connection;
use futures::TryFutureExt;
use image::imageops::FilterType;
use image::jpeg::JpegEncoder;
use image::{guess_format, DynamicImage, GenericImageView, ImageError};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::{self, remove_file};
use std::io::BufWriter;
use std::path::PathBuf;
use validator::Validate;

pub async fn list(state: Data<AppState>) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<_, APIError> { Ok(()) })
        .map_ok(ok_json)
        .map_err(APIError::from)
        .await
}

#[derive(Debug, FromMultipart, Validate)]
pub struct CreateGalleryItem {
    #[validate(length(max = 4096))]
    description: Option<String>,
    category: String,
    image: MultipartFile,
}

fn create_file<P: AsRef<std::path::Path>>(
    db: &state::Connection,
    input: NamedTempFile,
    destination: P,
    extension: Option<String>,
) -> Result<(File, PathBuf), APIError> {
    let size = input.as_file().metadata().unwrap().len();
    let f: File = diesel::insert_into(FilesDSL::files)
        .values((
            FilesDSL::bytes.eq(size as i64),
            FilesDSL::extension.eq(&extension),
        ))
        .get_result(db)?;
    let mut new_name = destination.as_ref().canonicalize().unwrap().to_path_buf();
    new_name.push(f.id.to_string());
    extension.map(|e| new_name.set_extension(e));
    input.persist_noclobber(&new_name).unwrap();
    Ok((f, new_name))
}

// https://support.squarespace.com/hc/en-us/articles/206542517-Formatting-your-images-for-display-on-the-web
static IMG_WIDTHS: [u32; 7] = [100, 300, 500, 750, 1000, 1500, 2500];
const JPEG_QUALITY: u8 = 80;

pub async fn create_item(
    state: Data<AppState>,
    form: ValidatedMultipartForm<CreateGalleryItem>,
) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<_, APIError> {
        // Check uploaded file is valid image
        let form = form.into_inner();
        let (img, format) = || -> Result<_, ImageError> {
            let img_bytes = fs::read(form.image.file.path()).unwrap();
            let format = guess_format(&img_bytes)?;
            let img = image::load_from_memory(&img_bytes)?;
            Ok((img, format))
        }()
        .map_err(|_| APIError::BadRequest {
            code: "BAD_IMAGE".to_string(),
            description: Some("The image file was not valid".to_string()),
        })?;

        let db = state.new_connection();
        let mut created = Vec::new();
        let res = db
            .transaction::<_, APIError, _>(|| {
                let ext = form.image.get_extension().map(|x| x.to_owned());
                let (original_file, original_path) = create_file(
                    &db,
                    form.image.file,
                    &state.settings.app.storage_folder,
                    ext,
                )?;
                created.push(original_path);

                let gallery_item: GalleryItem = diesel::insert_into(GalleryItemsDSL::gallery_items)
                    .values((
                        GalleryItemsDSL::description.eq(form.description),
                        GalleryItemsDSL::original_file_id.eq(original_file.id),
                        GalleryItemsDSL::position.eq("a"),
                        GalleryItemsDSL::category.eq(form.category),
                    ))
                    .get_result(&db)?;

                let widths: Vec<_> = IMG_WIDTHS.iter().filter(|w| **w <= img.width()).collect();
                let smaller_imgs: Vec<(NamedTempFile, DynamicImage)> = widths
                    .par_iter()
                    .map(|width| {
                        let resized = img.resize(**width, img.height(), FilterType::Triangle);
                        let tempf = NamedTempFile::new().unwrap();
                        let mut fout = BufWriter::new(tempf.as_file());
                        let mut encoder = JpegEncoder::new_with_quality(&mut fout, JPEG_QUALITY);
                        encoder.encode_image(&resized).unwrap();
                        drop(encoder);
                        drop(fout);
                        (tempf, resized)
                    })
                    .collect();

                for (tempfile, img) in smaller_imgs {
                    let (db_file, path) = create_file(
                        &db,
                        tempfile,
                        &state.settings.app.storage_folder,
                        Some("jpeg".to_string()),
                    )?;
                    created.push(path);
                    diesel::insert_into(GalleryFilesDSL::gallery_files)
                        .values((
                            GalleryFilesDSL::item_id.eq(gallery_item.id),
                            GalleryFilesDSL::file_id.eq(db_file.id),
                            GalleryFilesDSL::height.eq(img.height() as i32),
                            GalleryFilesDSL::width.eq(img.width() as i32),
                        ))
                        .execute(&db)?;
                }

                Ok(())
            })
            .map_err(|e| {
                created.iter().for_each(|f| {
                    remove_file(f.as_path()).unwrap();
                });
                e
            })?;

        Ok(())
    })
    .map_ok(ok_json)
    .map_err(APIError::from)
    .await
}

pub async fn delete_item(
    state: Data<AppState>,
    item_id: Path<i32>,
) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<_, APIError> { Ok(()) })
        .map_ok(ok_json)
        .map_err(APIError::from)
        .await
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateGalleryItem {
    #[validate(length(max = 4096))]
    description: Option<String>,
    category: String,
    after_id: Option<i32>,
}

pub async fn update_item(
    state: Data<AppState>,
    item_id: Path<i32>,
    form: ValidatedForm<UpdateGalleryItem>,
) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<_, APIError> { Ok(()) })
        .map_ok(ok_json)
        .map_err(APIError::from)
        .await
}
