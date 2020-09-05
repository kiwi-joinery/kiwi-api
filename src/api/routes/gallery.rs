use crate::api::errors::APIError;
use crate::api::ok_json;
use crate::state::AppState;
use actix_validated_forms::form::ValidatedForm;
use actix_validated_forms::multipart::{MultipartFile, ValidatedMultipartForm};
use actix_web::web::{Data, Path};
use actix_web::{web, HttpResponse};
use futures::TryFutureExt;
use image::GenericImageView;
use serde::{Deserialize, Serialize};
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

pub async fn create_item(
    state: Data<AppState>,
    form: ValidatedMultipartForm<CreateGalleryItem>,
) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<_, APIError> {
        let _img = image::open(form.image.file.path()).map_err(|e| APIError::BadRequest {
            code: "BAD_IMAGE".to_string(),
            description: Some("The image file was not valid".to_string()),
        })?;
        //
        // // The dimensions method returns the images width and height.
        // println!("dimensions {:?}", img.dimensions());
        //
        // // The color method returns the image's `ColorType`.
        // println!("{:?}", img.color());

        Ok(form.image.filename.clone())
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
