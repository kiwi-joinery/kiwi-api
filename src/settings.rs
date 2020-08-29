use config::{ConfigError, Config, Environment};
use serde::{Deserialize};

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

	pub fn new(_file: Option<&str>) -> Result<Self, ConfigError> {
		let mut s = Config::new();
		s.merge(Environment::new())?;
		s.try_into()
	}

}

impl Database {

	pub fn connection_url(&self) -> String {
		format!("postgres://{}:{}@{}:{}/{}", self.username, self.password, self.host, self.port, self.database)
	}

}
