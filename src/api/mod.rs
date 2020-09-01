//mod auth;
mod errors;
mod response;
//mod routes;
//mod token;

use actix_web::error::ResponseError;
use actix_web::web::{self, Data, PathConfig};
//use actix_web_httpauth::middleware::HttpAuthentication;
use crate::api::errors::APIError;
use crate::state::AppState;
use actix_validated_forms::form::ValidatedFormConfig;
use actix_validated_forms::query::ValidatedQueryConfig;

//Return a 404 if the path couldn't be found in a scope
fn scope(path: &str) -> actix_web::Scope {
    web::scope(path).default_service(web::route().to(|| APIError::NotFound.error_response()))
}

//Return a 405 if the method couldn't be found for a resource
fn resource(path: &str) -> actix_web::Resource {
    web::resource(path)
        .default_service(web::route().to(|| APIError::MethodNotAllowed.error_response()))
}

async fn index(_state: Data<AppState>) -> String {
    format!("Kiwi API")
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    //let auth_mw = HttpAuthentication::basic(auth::validator);
    cfg.service(
        scope("/")
            .data(PathConfig::default().error_handler(|e, _| APIError::from(e).into()))
            .data(ValidatedFormConfig::default().error_handler(|e, _| APIError::from(e).into()))
            .data(ValidatedQueryConfig::default().error_handler(|e, _| APIError::from(e).into()))
            .service(web::resource("").route(web::get().to(index))), // .service(scope("sessions")
                                                                     // 	.service(resource("login").route(web::post().to(routes::session::password_login)))
                                                                     // 	.service(resource("logout").route(web::delete().to(routes::session::logout)).wrap(auth_mw.clone()))
                                                                     // 	.service(resource("").route(web::get().to(routes::session::list)).wrap(auth_mw.clone()))
                                                                     // 	.service(resource("{session_id}").route(web::delete().to(routes::session::delete)).wrap(auth_mw.clone()))
                                                                     // )
                                                                     // .service(scope("users")
                                                                     // 	.service(resource("{user_id}")
                                                                     // 		.route(web::put().to_async(routes::users::update))
                                                                     // 	).wrap(auth_mw.clone())
                                                                     // )
    );
}
