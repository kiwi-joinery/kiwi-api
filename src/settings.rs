use config::{Config, ConfigError, Environment, File, FileFormat};
use lettre::smtp::authentication::Credentials;
use lettre::{smtp, ClientSecurity, ClientTlsParameters, SmtpClient, SmtpTransport};
use native_tls::TlsConnector;
use serde::Deserialize;
use std::path::Path;
use url::Url;
use validator::{Validate, ValidationError};

#[derive(Debug, Deserialize, Validate)]
pub struct App {
    pub port: u16,
    #[validate(custom = "validate_folder_path")]
    pub storage_folder: Box<Path>,
    pub contact_mailbox: String,
    pub password_reset_url: Url,
    pub api_url: Url,
}

fn validate_folder_path(path: &Box<Path>) -> Result<(), ValidationError> {
    std::fs::create_dir_all(path).map_err(|e| ValidationError::new("invalid folder path"))?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct Mailer {
    host: String,
    port: u16,
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
pub struct Database {
    host: String,
    port: u16,
    username: String,
    password: String,
    database: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct Settings {
    #[validate]
    pub app: App,
    pub database: Database,
    pub mailer: Mailer,
}

impl Settings {
    pub fn new(file: Option<&str>) -> Result<Self, ConfigError> {
        let mut s = Config::new();
        s.set_default("app.port", 9000)?;
        match file {
            None => {}
            Some(f) => {
                s.merge(File::with_name(f).format(FileFormat::Toml))?;
            }
        }
        s.merge(Environment::new())?;
        let r: Settings = s.try_into()?;
        r.validate()
            .map_err(|e| ConfigError::Message(format!("{}", e)))?;
        Ok(r)
    }
}

impl Database {
    pub fn connection_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database
        )
    }
}

impl Mailer {
    pub fn smtp_transport(&self) -> Result<SmtpTransport, smtp::error::Error> {
        let connector = TlsConnector::new().unwrap();
        let client = SmtpClient::new(
            format!("{}:{}", self.host, self.port),
            ClientSecurity::Required(ClientTlsParameters::new(self.host.clone(), connector)),
        )?
        .credentials(Credentials::new(self.email.clone(), self.password.clone()));
        Ok(SmtpTransport::new(client))
    }

    pub fn get_from_address(&self) -> &str {
        &self.email
    }
}
