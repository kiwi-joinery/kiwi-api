use crate::api::errors::APIError;
use crate::api::ok_json;
use crate::state::AppState;
use actix_validated_forms::form::ValidatedForm;
use actix_validated_forms::query::ValidatedQuery;
use actix_web::web::Data;
use actix_web::{web, HttpResponse};
use futures::TryFutureExt;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct ResetRequest {
    #[validate(email)]
    email: String,
}

pub async fn reset_request(
    state: Data<AppState>,
    email: ValidatedQuery<ResetRequest>,
) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<_, APIError> { Ok(()) })
        .map_ok(ok_json)
        .map_err(APIError::from)
        .await
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResetSubmit {
    token: String,
    email: String,
    #[validate(length(min = 8, max = 255))]
    new_password: String,
}

pub async fn reset_submit(
    state: Data<AppState>,
    form: ValidatedForm<ResetSubmit>,
) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<_, APIError> { Ok(()) })
        .map_ok(ok_json)
        .map_err(APIError::from)
        .await
}
