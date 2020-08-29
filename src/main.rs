#![allow(dead_code)]

// mod ext;
// mod api;
mod settings;
// mod state;
// mod schema;
// mod models;

// #[macro_use]
// extern crate diesel;
// #[macro_use]
// extern crate validator_derive;

// use actix_web::{middleware, App, HttpServer, HttpResponse, web};
// use ext::actix_web::HttpResponseExt;
use settings::Settings;
// use state::AppState;
use std::error::Error;
use std::env;
// use diesel::prelude::*;
// use diesel::r2d2::{self, ConnectionManager};

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

fn main() -> Result<(), Box<dyn Error>> {

	let matches = parse_cli();
	if cfg!(debug_assertions) {println!("THIS IS A DEBUG BUILD")};
	let settings = Settings::new(matches.value_of("config"))?;
	//
	// //Configure logging
	// let level = if cfg!(debug_assertions) {"info"} else {"error"};
	// env::set_var("RUST_LOG", format!("actix_web={}", level));
	// env_logger::init();
	//
	// //Connect to database
	// let manager = ConnectionManager::<PgConnection>::new(settings.database.connection_url());
	// let pool = r2d2::Pool::builder().build(manager)?;
	//
	// //Create the application state
	// let state = AppState::new(settings, pool);
	//
	// //Start the HTTP Server
	// let address = format!("127.0.0.1:{}", state.settings.app.port);
	// println!("Starting server on: http://{}", address);
	// HttpServer::new(move || {
	// 	App::new()
	// 		.data(state.clone())
	// 		.wrap(middleware::Logger::default())
	// 		.configure(api::configure)
	// 		.service(web::resource("/").route(web::get().to(|| HttpResponse::found_to("app"))))
	// }).bind(address)?.run()?;

	Ok(())
}
