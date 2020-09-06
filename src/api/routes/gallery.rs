use crate::api::errors::APIError;
use crate::api::ok_json;
use crate::models::{File, GalleryFile, GalleryItem};
use crate::schema::files::dsl as Files;
use crate::schema::gallery_files::dsl as GalleryFiles;
use crate::schema::gallery_items::dsl as GalleryItems;
use crate::state::{self, AppState};
use actix_validated_forms::form::ValidatedForm;
use actix_validated_forms::multipart::{
    MultipartFile, MultipartTypeFromString, ValidatedMultipartForm,
};
use actix_validated_forms::tempfile::NamedTempFile;
use actix_web::web::{Data, Path};
use actix_web::{web, HttpResponse};
use diesel::prelude::*;
use diesel::Connection;
use futures::TryFutureExt;
use image::imageops::FilterType;
use image::jpeg::JpegEncoder;
use image::{guess_format, DynamicImage, GenericImageView, ImageError};
use itertools::Itertools;
use rayon::prelude::*;
use serde::export::Formatter;
use serde::{Deserialize, Serialize};
use std::fs::{self, remove_file};
use std::io::BufWriter;
use std::path::PathBuf;
use std::str::FromStr;
use url::Url;
use validator::Validate;

#[derive(Serialize)]
pub struct GalleryItemResponse {
    pub id: i32,
    pub description: Option<String>,
    pub position: String,
    pub category: String,
    pub files: Vec<GalleryFileResponse>,
}

#[derive(Serialize)]
pub struct GalleryFileResponse {
    url: Url,
    height: i32,
    width: i32,
    bytes: i64,
}

fn create_file_url(settings: &crate::settings::Settings, file: &File) -> Url {
    let mut s = format!("files/{}", file.id);
    match file.extension.as_ref() {
        None => {}
        Some(e) => s.push_str(format!(".{}", e).as_str()),
    }
    settings.app.api_url.join(s.as_str()).unwrap()
}

pub async fn list(state: Data<AppState>) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<_, APIError> {
        let db = state.new_connection();

        let items: Vec<(GalleryItem, Option<(GalleryFile, File)>)> = GalleryItems::gallery_items
            .left_join(GalleryFiles::gallery_files.inner_join(Files::files))
            .get_results(&db)?;

        let mut output = Vec::new();
        for (_, group) in &items.into_iter().group_by(|x| x.0.id) {
            let mut files = Vec::new();
            let group = group.collect_vec();
            for (_, f) in &group {
                match f {
                    None => {}
                    Some((g, f)) => files.push(GalleryFileResponse {
                        url: create_file_url(&state.settings, f),
                        height: g.height,
                        width: g.width,
                        bytes: f.bytes,
                    }),
                }
            }
            let first = &group.first().unwrap().0;
            output.push(GalleryItemResponse {
                id: first.id,
                description: first.description.clone(),
                position: first.position.clone(),
                category: first.category.clone(),
                files,
            });
        }
        Ok(output)
    })
    .map_ok(ok_json)
    .map_err(APIError::from)
    .await
}

#[derive(Debug, Serialize)]
enum Category {
    Staircases,
    Windows,
    Doors,
    Other,
}

#[derive(Debug, FromMultipart, Validate)]
pub struct CreateGalleryItem {
    #[validate(length(max = 4096))]
    description: Option<String>,
    category: Category,
    image: MultipartFile,
}

fn create_file<P: AsRef<std::path::Path>>(
    db: &state::Connection,
    input: NamedTempFile,
    destination: P,
    extension: Option<String>,
) -> Result<(File, PathBuf), APIError> {
    let size = input.as_file().metadata().unwrap().len();
    let f: File = diesel::insert_into(Files::files)
        .values((
            Files::bytes.eq(size as i64),
            Files::extension.eq(&extension),
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
        let img = || -> Result<_, ImageError> {
            let img_bytes = fs::read(form.image.file.path()).unwrap();
            let format = guess_format(&img_bytes)?;
            let img = image::load_from_memory(&img_bytes)?;
            Ok(img)
        }()
        .map_err(|_| APIError::BadRequest {
            code: "BAD_IMAGE".to_string(),
            description: Some("The image file was not valid".to_string()),
        })?;

        let db = state.new_connection();
        let mut created = Vec::new();
        let gallery_item_id = db
            .transaction::<_, APIError, _>(|| {
                let ext = form.image.get_extension().map(|x| x.to_owned());
                let (original_file, original_path) = create_file(
                    &db,
                    form.image.file,
                    &state.settings.app.storage_folder,
                    ext,
                )?;
                created.push(original_path);

                let gallery_item: GalleryItem = diesel::insert_into(GalleryItems::gallery_items)
                    .values((
                        GalleryItems::description.eq(form.description),
                        GalleryItems::original_file_id.eq(original_file.id),
                        GalleryItems::position.eq("a"),
                        GalleryItems::category.eq(form.category.to_string()),
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
                        Some("jpg".to_string()),
                    )?;
                    created.push(path);
                    diesel::insert_into(GalleryFiles::gallery_files)
                        .values(&GalleryFile {
                            item_id: gallery_item.id,
                            file_id: db_file.id,
                            height: img.height() as i32,
                            width: img.width() as i32,
                        })
                        .execute(&db)?;
                }
                Ok(gallery_item.id)
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
    web::block(move || -> Result<_, APIError> {
        let db = state.new_connection();
        let item: Vec<(GalleryItem, Option<GalleryFile>, Option<File>)> =
            GalleryItems::gallery_items
                .find(item_id.into_inner())
                .left_join(GalleryFiles::gallery_files)
                .left_join(Files::files)
                .get_results(&db)?;

        // db.transaction::<_, APIError, _>(|| {
        //     diesel::delete(GalleryFiles::gallery_files.filter(GalleryFiles::item_id.eq(item.id)))
        //         .execute(&db)?;
        //     diesel::delete(GalleryItems::gallery_items.filter(GalleryItems::id.eq(item.id)))
        //         .execute(&db)?;
        //
        //     Ok(())
        // });
        Ok(())
    })
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

impl MultipartTypeFromString for Category {}
impl FromStr for Category {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "STAIRCASES" => Ok(Self::Staircases),
            "WINDOWS" => Ok(Self::Windows),
            "DOORS" => Ok(Self::Doors),
            "OTHER" => Ok(Self::Other),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Category::Staircases => f.write_str("STAIRCASES"),
            Category::Windows => f.write_str("WINDOWS"),
            Category::Doors => f.write_str("DOORS"),
            Category::Other => f.write_str("OTHER"),
        }
    }
}
