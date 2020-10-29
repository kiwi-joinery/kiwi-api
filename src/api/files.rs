use crate::api::errors::APIError;
use crate::models::File;
use crate::schema::files::dsl as F;
use crate::settings::Settings;
use crate::state::Connection;
use actix_validated_forms::tempfile::NamedTempFile;
use diesel::prelude::*;
use std::path::PathBuf;
use url::Url;

impl File {
    // URL this file can be accessed from
    pub fn get_public_url(&self, settings: &Settings) -> Url {
        let mut s = format!("files/{}", self.id);
        match self.extension.as_ref() {
            None => {}
            Some(e) => s.push_str(format!(".{}", e).as_str()),
        }
        settings.app.api_url.join(s.as_str()).unwrap()
    }

    // Where this file is stored on disk
    pub fn get_storage_path(&self, settings: &Settings) -> PathBuf {
        let mut p = settings
            .app
            .storage_folder
            .as_ref()
            .canonicalize()
            .unwrap()
            .to_path_buf();
        p.push(self.id.to_string());
        self.extension.as_ref().map(|e| p.set_extension(e));
        p
    }

    // Create a database entry and move into storage folder
    pub fn create(
        db: &Connection,
        settings: &Settings,
        input: NamedTempFile,
        extension: Option<String>,
    ) -> Result<File, APIError> {
        let size = input.as_file().metadata().unwrap().len();
        let f: File = diesel::insert_into(F::files)
            .values((F::bytes.eq(size as i64), F::extension.eq(&extension)))
            .get_result(db)?;
        let mut new_name = settings
            .app
            .storage_folder
            .as_ref()
            .canonicalize()
            .unwrap()
            .to_path_buf();
        new_name.push(f.id.to_string());
        extension.map(|e| new_name.set_extension(e));
        input.persist_noclobber(&new_name).unwrap();
        Ok(f)
    }

    // Remove from storage folder
    pub fn delete_from_disk(self, settings: &Settings) {
        std::fs::remove_file(self.get_storage_path(settings)).unwrap()
    }
}
