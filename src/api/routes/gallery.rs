use crate::api::errors::APIError;
use crate::api::ok_json;
use crate::ext::image_exif::read_image;
use crate::models::{File, GalleryFile, GalleryItem, GalleryItemChange};
use crate::schema::files::dsl as Files;
use crate::schema::gallery_files::dsl as GalleryFiles;
use crate::schema::gallery_items::dsl as GalleryItems;
use crate::state::AppState;
use actix_validated_forms::form::ValidatedForm;
use actix_validated_forms::multipart::{MultipartFile, ValidatedMultipartForm};
use actix_validated_forms::tempfile::NamedTempFile;
use actix_web::web::{Data, Path};
use actix_web::{web, HttpResponse};
use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel::Connection;
use enum_iterator::IntoEnumIterator;
use futures::TryFutureExt;
use image::imageops::FilterType;
use image::jpeg::JpegEncoder;
use image::{DynamicImage, GenericImageView};
use itertools::Itertools;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::BufWriter;
use std::str::FromStr;
use url::Url;
use validator::{Validate, ValidationError};

#[derive(Debug, Serialize, Deserialize, IntoEnumIterator, PartialEq, Eq, Hash)]
#[serde(rename_all = "UPPERCASE")]
pub enum Category {
    Staircases,
    Windows,
    Doors,
    Other,
}

#[derive(Serialize)]
pub struct GalleryItemResponse {
    pub id: i32,
    pub description: String,
    pub category: Category,
    pub files: Vec<GalleryFileResponse>,
}

#[derive(Serialize)]
pub struct GalleryFileResponse {
    url: Url,
    height: i32,
    width: i32,
    bytes: i64,
}

pub async fn list(state: Data<AppState>) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<_, APIError> {
        let db = state.new_connection();

        let items: Vec<(GalleryItem, Option<(GalleryFile, File)>)> = GalleryItems::gallery_items
            .left_join(GalleryFiles::gallery_files.inner_join(Files::files))
            .order((GalleryItems::position.asc(), GalleryFiles::width.asc()))
            .get_results(&db)?;

        // Be careful group by behaviour is weird, but will work here because of ordering
        let mut grouped_by_image = Vec::new();
        for (_, group) in &items.into_iter().group_by(|x| x.0.id) {
            let mut files = Vec::new();
            let group = group.collect_vec();
            for (_, f) in &group {
                match f {
                    None => {}
                    Some((g, f)) => files.push(GalleryFileResponse {
                        url: f.get_public_url(&state.settings),
                        height: g.height,
                        width: g.width,
                        bytes: f.bytes,
                    }),
                }
            }
            let first = &group.first().unwrap().0;
            grouped_by_image.push(GalleryItemResponse {
                id: first.id,
                description: first.description.clone(),
                category: first.category.parse().unwrap(),
                files,
            });
        }

        let mut grouped_by_category = HashMap::new();
        for i in Category::into_enum_iter() {
            grouped_by_category.insert(i, Vec::new());
        }
        for i in grouped_by_image.into_iter() {
            grouped_by_category.get_mut(&i.category).unwrap().push(i);
        }
        Ok(grouped_by_category)
    })
    .map_ok(ok_json)
    .map_err(APIError::from)
    .await
}

pub async fn get_item(item_id: Path<i32>, state: Data<AppState>) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<_, APIError> {
        let db = state.new_connection();
        let mut items: Vec<(GalleryItem, Option<(GalleryFile, File)>)> =
            GalleryItems::gallery_items
                .left_join(GalleryFiles::gallery_files.inner_join(Files::files))
                .filter(GalleryItems::id.eq(item_id.into_inner()))
                .get_results(&db)?;
        if items.len() < 1 {
            return Err(APIError::NotFound);
        }
        let mut files = Vec::new();
        for (_, f) in &items {
            match f {
                None => {}
                Some((g, f)) => files.push(GalleryFileResponse {
                    url: f.get_public_url(&state.settings),
                    height: g.height,
                    width: g.width,
                    bytes: f.bytes,
                }),
            }
        }
        let first = items.remove(0).0;
        Ok(GalleryItemResponse {
            id: first.id,
            description: first.description,
            category: first.category.parse().unwrap(),
            files,
        })
    })
    .map_ok(ok_json)
    .err_into()
    .await
}

#[derive(Debug, FromMultipart, Validate)]
pub struct CreateGalleryItem {
    #[validate(length(max = 4096))]
    description: String,
    category: Category,
    image: MultipartFile,
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
        let img_bytes = std::fs::read(form.image.file.path()).unwrap();
        let img = read_image(&img_bytes).map_err(|_| APIError::BadRequest {
            code: "BAD_IMAGE".to_string(),
            description: Some("The image file was not valid".to_string()),
        })?;

        let db = state.new_connection();
        let mut created = Vec::new();
        db.transaction::<_, APIError, _>(|| {
            let ext = form.image.get_extension().map(|x| x.to_owned());
            let original_file = File::create(&db, &state.settings, form.image.file, ext)?;
            let original_file_id = original_file.id;
            created.push(original_file);

            let pos = match GalleryItems::gallery_items
                .order(GalleryItems::position.desc())
                .limit(1)
                .get_result::<GalleryItem>(&db)
                .optional()?
            {
                None => BigDecimal::from(100),
                Some(x) => x.position + BigDecimal::from(100),
            };

            let gallery_item: GalleryItem = diesel::insert_into(GalleryItems::gallery_items)
                .values((
                    GalleryItems::description.eq(form.description),
                    GalleryItems::original_file_id.eq(original_file_id),
                    GalleryItems::position.eq(pos),
                    GalleryItems::category.eq(form.category.serialize()),
                ))
                .get_result(&db)?;

            let mut widths: Vec<u32> = IMG_WIDTHS
                .to_vec()
                .into_iter()
                .filter(|w| w <= &img.width())
                .collect();
            if *widths.iter().max().unwrap_or(&(0 as u32)) < img.width() {
                widths.push(img.width())
            }

            let smaller_imgs: Vec<(NamedTempFile, DynamicImage)> = widths
                .par_iter()
                .map(|width| {
                    let resized = img.resize(*width, img.height(), FilterType::Triangle);
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
                let db_file =
                    File::create(&db, &state.settings, tempfile, Some("jpg".to_string()))?;
                let db_file_id = db_file.id;
                created.push(db_file);
                diesel::insert_into(GalleryFiles::gallery_files)
                    .values(&GalleryFile {
                        item_id: gallery_item.id,
                        file_id: db_file_id,
                        height: img.height() as i32,
                        width: img.width() as i32,
                    })
                    .execute(&db)?;
            }
            Ok(gallery_item.id)
        })
        .map_err(|e| {
            created.into_iter().for_each(|f| {
                f.delete_from_disk(&state.settings);
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
        let (item, original_file): (GalleryItem, File) = GalleryItems::gallery_items
            .find(item_id.into_inner())
            .inner_join(Files::files)
            .get_result(&db)?;

        let mut files: Vec<File> = GalleryItems::gallery_items
            .find(item.id)
            .inner_join(GalleryFiles::gallery_files.inner_join(Files::files))
            .get_results::<(GalleryItem, (GalleryFile, File))>(&db)?
            .into_iter()
            .map(|(_, (_, f))| f)
            .collect_vec();
        files.push(original_file);

        db.transaction::<_, APIError, _>(|| {
            // Delete gallery file mappings
            diesel::delete(GalleryFiles::gallery_files.filter(GalleryFiles::item_id.eq(item.id)))
                .execute(&db)?;
            // Delete gallery item
            diesel::delete(GalleryItems::gallery_items.filter(GalleryItems::id.eq(item.id)))
                .execute(&db)?;
            // Delete file records
            for f in files.iter() {
                diesel::delete(Files::files.filter(Files::id.eq(f.id))).execute(&db)?;
            }
            Ok(())
        })?;

        // Delete files from disk
        files.into_iter().for_each(|f| {
            f.delete_from_disk(&state.settings);
        });

        Ok(())
    })
    .map_ok(ok_json)
    .map_err(APIError::from)
    .await
}

#[derive(Debug, Deserialize, Validate)]
#[validate(schema(function = "validate_update_gallery_item"))]
pub struct UpdateGalleryItem {
    #[validate(length(max = 4096))]
    description: String,
    category: Category,
    move_after_id: Option<i32>,
    #[serde(default = "serde_false")]
    move_to_front: bool,
}

fn serde_false() -> bool {
    false
}

fn validate_update_gallery_item(x: &UpdateGalleryItem) -> Result<(), ValidationError> {
    if x.move_after_id.is_some() && x.move_to_front {
        return Err(ValidationError::new(
            "Cannot set both move_after_id and move_to_front",
        ));
    }
    Ok(())
}

pub async fn update_item(
    state: Data<AppState>,
    item_id: Path<i32>,
    form: ValidatedForm<UpdateGalleryItem>,
) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<_, APIError> {
        let db = state.new_connection();
        let target: GalleryItem = GalleryItems::gallery_items
            .find(item_id.into_inner())
            .get_result(&db)?;

        let new_pos = if form.move_to_front {
            // Insert halfway between 0 and the first current position
            GalleryItems::gallery_items
                .filter(GalleryItems::category.eq(form.category.serialize()))
                .limit(1)
                .order(GalleryItems::position.asc())
                .get_result::<GalleryItem>(&db)
                .optional()?
                .map(|x| (BigDecimal::from(0) + x.position) / BigDecimal::from(2))
        } else if let Some(after_id) = form.move_after_id {
            // Find the position of the item it is being inserted after
            let after: GalleryItem = GalleryItems::gallery_items
                .filter(GalleryItems::id.eq(after_id))
                .filter(GalleryItems::category.eq(form.category.serialize()))
                .get_result::<GalleryItem>(&db)
                .optional()?
                .ok_or(APIError::BadRequest {
                    code: "BAD_REQUEST".to_string(),
                    description: Some("after_id is invalid".to_string()),
                })?;
            // Find the position of the item it is being inserted before
            let before: Option<GalleryItem> = GalleryItems::gallery_items
                .filter(GalleryItems::position.gt(&after.position))
                .filter(GalleryItems::category.eq(form.category.serialize()))
                .order(GalleryItems::position.asc())
                .get_result(&db)
                .optional()?;
            Some(match before {
                None => after.position + BigDecimal::from(100),
                Some(before) => (after.position + before.position) / BigDecimal::from(2),
            })
        } else if target.category != form.category.serialize() {
            // If the category is being changed, then insert after the last current position
            Some(
                GalleryItems::gallery_items
                    .filter(GalleryItems::category.eq(form.category.serialize()))
                    .limit(1)
                    .order(GalleryItems::position.desc())
                    .get_result::<GalleryItem>(&db)
                    .optional()?
                    .map(|x| x.position + BigDecimal::from(100))
                    .unwrap_or(BigDecimal::from(100)),
            )
        } else {
            // Item is not changing position or category
            None
        };

        let query = diesel::update(&target)
            .set(&GalleryItemChange {
                description: form.description.clone(),
                position: new_pos,
                category: form.category.serialize(),
            })
            .execute(&db)?;
        if query < 1 {
            return Err(APIError::NotFound);
        }
        Ok(())
    })
    .map_ok(ok_json)
    .map_err(APIError::from)
    .await
}

impl FromStr for Category {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_plain::from_str::<Self>(s).map_err(|_| ())
    }
}

impl Category {
    fn serialize(&self) -> String {
        serde_plain::to_string(&self).unwrap()
    }
}
