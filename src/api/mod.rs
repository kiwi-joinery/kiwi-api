mod actix;
mod auth;
mod errors;
mod files;
mod routes;
mod token;

use crate::api::errors::APIError;
use crate::state::AppState;
use actix_files::Files;
use actix_ratelimit::{MemoryStore, MemoryStoreActor, RateLimiter};
use actix_validated_forms::form::ValidatedFormConfig;
use actix_validated_forms::multipart::ValidatedMultipartFormConfig;
use actix_validated_forms::query::ValidatedQueryConfig;
use actix_web::error::ResponseError;
use actix_web::web::{self, Data, PathConfig};
use actix_web::HttpResponse;
use actix_web_httpauth::middleware::HttpAuthentication;
use serde::Serialize;
use std::time::Duration;

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

pub fn configure(cfg: &mut web::ServiceConfig, state: AppState) {
    let auth_mw = HttpAuthentication::basic(auth::validator);
    let rl_store = MemoryStore::new();
    cfg.service(
        scope("/")
            .app_data(PathConfig::default().error_handler(|e, _| APIError::from(e).into()))
            .app_data(ValidatedFormConfig::default().error_handler(|e, _| APIError::from(e).into()))
            .app_data(
                ValidatedQueryConfig::default().error_handler(|e, _| APIError::from(e).into()),
            )
            .app_data(
                ValidatedMultipartFormConfig::default()
                    .error_handler(|e, _| APIError::from(e).into()),
            )
            .service(web::resource("").route(web::get().to(index)))
            .service(
                scope("sessions")
                    .service(
                        resource("login")
                            .route(web::post().to(routes::session::password_login))
                            .wrap(
                                RateLimiter::new(MemoryStoreActor::from(rl_store.clone()).start())
                                    .with_interval(Duration::from_secs(120))
                                    .with_max_requests(5),
                            ),
                    )
                    .service(
                        resource("logout")
                            .route(web::delete().to(routes::session::logout))
                            .wrap(auth_mw.clone()),
                    )
                    .service(
                        resource("")
                            .route(web::get().to(routes::session::list))
                            .wrap(auth_mw.clone()),
                    )
                    .service(
                        resource("{session_id}")
                            .route(web::delete().to(routes::session::delete))
                            .wrap(auth_mw.clone()),
                    ),
            )
            .service(
                scope("users")
                    .service(
                        resource("")
                            .route(web::get().to(routes::users::list))
                            .route(web::post().to(routes::users::create)),
                    )
                    .service(
                        resource("{user_id}")
                            .route(web::get().to(routes::users::get))
                            .route(web::put().to(routes::users::update))
                            .route(web::delete().to(routes::users::delete)),
                    )
                    .wrap(auth_mw.clone()),
            )
            .service(
                resource("contact")
                    .route(web::post().to(routes::contact::contact_form))
                    .wrap(
                        RateLimiter::new(MemoryStoreActor::from(rl_store.clone()).start())
                            .with_interval(Duration::from_secs(120))
                            .with_max_requests(3),
                    ),
            )
            .service(
                scope("password_reset")
                    .service(
                        resource("request").route(web::post().to(routes::password_reset::request)),
                    )
                    .service(
                        resource("submit").route(web::post().to(routes::password_reset::submit)),
                    )
                    .wrap(
                        RateLimiter::new(MemoryStoreActor::from(rl_store.clone()).start())
                            .with_interval(Duration::from_secs(120))
                            .with_max_requests(3),
                    ),
            )
            .service(
                scope("gallery")
                    .service(resource("list").route(web::get().to(routes::gallery::list)))
                    .service(
                        scope("")
                            .service(
                                resource("").route(web::post().to(routes::gallery::create_item)),
                            )
                            .service(
                                resource("{item_id}")
                                    .route(web::put().to(routes::gallery::update_item))
                                    .route(web::delete().to(routes::gallery::delete_item)),
                            )
                            .wrap(auth_mw.clone()),
                    ),
            )
            .service(Files::new(
                "/files",
                state
                    .settings
                    .app
                    .storage_folder
                    .canonicalize()
                    .unwrap()
                    .as_os_str(),
            )),
    );
}

pub fn ok_json<T: Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok().json(data)
}
