#![allow(dead_code)]

// mod ext;
mod api;
mod settings;
mod state;
// mod schema;
// mod models;

// #[macro_use]
// extern crate diesel;
// #[macro_use]
// extern crate validator_derive;

#[macro_use]
extern crate diesel_migrations;
embed_migrations!();

use actix_web::{middleware, App, HttpServer, HttpResponse, web};
use settings::Settings;
use state::AppState;
use std::error::Error;
use std::env;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};


fn parse_cli() -> clap::ArgMatches<'static> {
	clap::App::new(env!("CARGO_PKG_NAME"))
		.version(env!("CARGO_PKG_VERSION"))
		.author(env!("CARGO_PKG_AUTHORS"))
		.arg(clap::Arg::with_name("config")
			.short("c")
			.long("config")
			.takes_value(true)
			.value_name("FILE")
			.help("Sets the config file path")
		).get_matches()
}

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn Error>> {
	let matches = parse_cli();
	if cfg!(debug_assertions) { println!("THIS IS A DEBUG BUILD") };
	let settings = Settings::new(matches.value_of("config"))?;

	let level = if cfg!(debug_assertions) { "info" } else { "error" };
	env::set_var("RUST_LOG", format!("actix_web={}", level));
	env_logger::init();

	let manager = ConnectionManager::<PgConnection>::new(settings.database.connection_url());
	let pool = r2d2::Pool::builder().build(manager)?;
	let state = AppState::new(settings, pool);

	embedded_migrations::run_with_output(&state.new_connection(), &mut std::io::stdout())?;

	let address = format!("127.0.0.1:{}", state.settings.app.port);
	println!("Starting server on: http://{}", address);
	HttpServer::new(move || {
		App::new()
			.data(state.clone())
			.wrap(middleware::Logger::default())
			.configure(api::configure)
	}).bind(address)?.run().await?;

	Ok(())
}
