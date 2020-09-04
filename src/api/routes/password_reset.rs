use crate::api::errors::APIError;
use crate::api::ok_json;
use crate::api::routes::session::AUTH_TOKEN_BYTES;
use crate::api::token::generate_token;
use crate::models::User;
use crate::schema::users::dsl as U;
use crate::state::AppState;
use actix_validated_forms::form::ValidatedForm;
use actix_web::web::Data;
use actix_web::{web, HttpResponse};
use bcrypt::{hash, DEFAULT_COST};
use diesel::prelude::*;
use futures::TryFutureExt;
use lettre::Transport;
use lettre_email::EmailBuilder;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct ResetRequest {
    #[validate(email)]
    email: String,
}

pub async fn reset_request(
    state: Data<AppState>,
    email: ValidatedForm<ResetRequest>,
) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<_, APIError> {
        let db = state.new_connection();
        let mut mailer = state.settings.mailer.smtp_transport()?;

        let user: User = match U::users
            .filter(U::email.eq(&email.email))
            .first::<User>(&db)
            .optional()?
        {
            Some(r) => r,
            None => {
                return Err(APIError::BadRequest {
                    code: "INCORRECT_EMAIL".to_string(),
                    description: Some("Email address not found".to_string()),
                })
            }
        };

        let token = match user.password_reset_token {
            Some(t) => t,
            None => {
                let token = generate_token(AUTH_TOKEN_BYTES);
                diesel::update(&user)
                    .set(U::password_reset_token.eq(&token))
                    .execute(&db)?;
                token
            }
        };

        let email = EmailBuilder::new()
            .to(email.email.as_str())
            .from(state.settings.mailer.get_from_address())
            .reply_to("noreply@kiwijoinerydevon.co.uk")
            .subject("Kiwi Website Password Reset")
            .body(token)
            .build()
            .unwrap();
        mailer.send(email.into())?;

        Ok(())
    })
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
    web::block(move || -> Result<_, APIError> {
        let db = state.new_connection();

        let user: User = match U::users
            .filter(U::email.eq(&form.email))
            .filter(U::password_reset_token.eq(&form.token))
            .first::<User>(&db)
            .optional()?
        {
            Some(r) => r,
            None => return Err(APIError::IncorrectCredentials),
        };

        let new = hash(&form.new_password, DEFAULT_COST)?;

        diesel::update(&user)
            .set(U::password_hash.eq(&new))
            .execute(&db)?;

        Ok(())
    })
    .map_ok(ok_json)
    .map_err(APIError::from)
    .await
}
