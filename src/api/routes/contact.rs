use crate::api::errors::APIError;
use crate::api::response::ok_response;
use crate::state::AppState;
use actix_validated_forms::form::ValidatedForm;
use actix_web::web::Data;
use actix_web::{web, HttpResponse};
use futures::TryFutureExt;
use lettre::Transport;
use lettre_email::EmailBuilder;
use linkify::{LinkFinder, LinkKind};
use serde::Deserialize;
use validator::{Validate, ValidationError};

#[derive(Debug, Deserialize, Validate)]
pub struct Contact {
    #[validate(length(min = 1, max = 100))]
    name: String,
    #[validate(email)]
    email: String,
    #[validate(length(min = 10, max = 4000), custom = "validate_no_urls")]
    message: String,
}

fn validate_no_urls(message: &str) -> Result<(), ValidationError> {
    let mut finder = LinkFinder::new();
    finder.kinds(&[LinkKind::Url]);
    if finder.links(message).count() > 0 {
        return Err(ValidationError::new("Your message must not include URLs"));
    }
    Ok(())
}

pub async fn contact_form(
    state: Data<AppState>,
    contact: ValidatedForm<Contact>,
) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<_, APIError> {
        let mut mailer = state.settings.mailer.smtp_transport()?;
        let msg = format!(
            "<body>\n
            <h3>Message:</h3><p>{}</p>\n
            <h3>Sent by: </h3><p>{}</p>\n
            <h3>Sent from: </h3><p>{}</p>\n
            <br><br><hr>\n
            </body>\n\
            </html>",
            contact.message, contact.name, contact.email
        );
        let email = EmailBuilder::new()
            .to(state.settings.app.contact_mailbox.as_str())
            .from(state.settings.mailer.email.as_str())
            .reply_to(format!("{} <{}>", &contact.name, &contact.email))
            .subject("Message from Kiwi Website contact form")
            .html(msg)
            .build()
            .unwrap();
        mailer.send(email.into())?;
        Ok(())
    })
    .map_ok(ok_response)
    .map_err(APIError::from)
    .await
}
