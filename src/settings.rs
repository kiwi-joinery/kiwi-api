use config::{ConfigError, Config, Environment, File, FileFormat};
use serde::{Deserialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct App {
	pub port: u16,
	pub storage: String,
}

#[derive(Debug, Deserialize)]
pub struct Database {
	host: String,
	port: u16,
	username: String,
	password: String,
	database: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
	pub app: App,
	pub database: Database,
}

impl Settings {

	pub fn new(file: Option<&str>) -> Result<Self, ConfigError> {
		let mut s = Config::new();
		s.set_default("app.port", 9000);
		match file {
			None => {},
			Some(f) => {
				s.merge(File::with_name(f).format(FileFormat::Toml))?;
			},
		}
		s.merge(Environment::new());
		s.try_into()
	}

}

impl Database {

	pub fn connection_url(&self) -> String {
		format!("postgres://{}:{}@{}:{}/{}", self.username, self.password, self.host, self.port, self.database)
	}

}
