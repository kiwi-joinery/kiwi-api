mod api;
mod ext;
mod models;
mod schema;
mod settings;
mod state;

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate validator_derive;
#[macro_use]
extern crate diesel_migrations;
embed_migrations!();
#[macro_use]
extern crate actix_validated_forms;

use actix_cors::Cors;
use actix_web::{middleware, App, HttpServer};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use settings::Settings;
use state::AppState;
use std::env;
use std::error::Error;

fn parse_cli() -> clap::ArgMatches<'static> {
    clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .arg(
            clap::Arg::with_name("config")
                .short("c")
                .long("config")
                .takes_value(true)
                .value_name("FILE")
                .help("Sets the config file path"),
        )
        .get_matches()
}

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let matches = parse_cli();
    if cfg!(debug_assertions) {
        println!("THIS IS A DEBUG BUILD")
    };
    let settings = Settings::new(matches.value_of("config"))?;

    let manager = ConnectionManager::<PgConnection>::new(settings.database.connection_url());
    let pool = r2d2::Pool::builder().build(manager)?;
    let state = AppState::new(settings, pool);

    embedded_migrations::run_with_output(&state.new_connection(), &mut std::io::stdout())?;

    let address = format!("0.0.0.0:{}", state.settings.app.port);
    println!("Starting server on port {}", state.settings.app.port);
    HttpServer::new(move || {
        let cors = Cors::new()
            .allowed_origin("https://www.kiwijoinerydevon.co.uk")
            .allowed_origin("https://admin.kiwijoinerydevon.co.uk")
            .allowed_origin("https://kiwijoinerydevon.co.uk")
            .allowed_origin("https://kiwi-joinery.github.io/")
            .max_age(3600)
            .finish();
        App::new()
            .data(state.clone())
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .configure(|c| api::configure(c, state.clone()))
    })
    .bind(address)?
    .run()
    .await?;

    Ok(())
}
