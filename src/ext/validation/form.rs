use validator::{Validate, ValidationErrors};
use std::{ops, fmt};
use actix_web::{FromRequest, HttpRequest, ResponseError};
use serde::de::DeserializeOwned;
use futures::Future;
use actix_web::dev::{UrlEncoded, Payload};
use actix_web::error::UrlencodedError;
use std::rc::Rc;

#[derive(Debug)]
pub enum ValidatedFormError {
    Deserialization(UrlencodedError),
    Validation(ValidationErrors),
}

impl std::error::Error for ValidatedFormError {}
impl ResponseError for ValidatedFormError {}

impl fmt::Display for ValidatedFormError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValidatedFormError::Validation(e) => e.fmt(f),
            ValidatedFormError::Deserialization(e) => e.fmt(f)
        }
    }
}

pub struct ValidatedForm<T: Validate>(pub T);

impl<T: Validate> ValidatedForm<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: Validate> ops::Deref for ValidatedForm<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Validate> ops::DerefMut for ValidatedForm<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> FromRequest for ValidatedForm<T>
    where
        T: Validate + DeserializeOwned + 'static,
{
    type Error = actix_web::Error;
    type Future = Box<Future<Item = Self, Error = Self::Error>>;
    type Config = ValidatedFormConfig;

    #[inline]
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req2 = req.clone();
        let config = req.app_data::<ValidatedFormConfig>()
            .map(|c| c.clone())
            .unwrap_or(ValidatedFormConfig::default());

        Box::new(
            UrlEncoded::new(req, payload)
                .limit(config.limit)
                .map_err(move |e| ValidatedFormError::Deserialization(e))
                .and_then(|c: T| {
                    c.validate().map(|_| c).map_err(|e| ValidatedFormError::Validation(e))
                })
                .map_err(move |e| {
                    if let Some(err) = config.error_handler {
                        (*err)(e, &req2)
                    } else {
                        e.into()
                    }
                })
                .map(ValidatedForm)
        )
    }
}

impl<T: Validate + fmt::Debug> fmt::Debug for ValidatedForm<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: Validate + fmt::Display> fmt::Display for ValidatedForm<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone)]
pub struct ValidatedFormConfig {
    limit: usize,
    error_handler: Option<Rc<Fn(ValidatedFormError, &HttpRequest) -> actix_web::Error>>,
}

impl ValidatedFormConfig {
    /// Change max size of payload. By default max size is 16Kb
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set custom error handler
    pub fn error_handler<F>(mut self, f: F) -> Self
        where
            F: Fn(ValidatedFormError, &HttpRequest) -> actix_web::Error + 'static,
    {
        self.error_handler = Some(Rc::new(f));
        self
    }
}

impl Default for ValidatedFormConfig {
    fn default() -> Self {
        ValidatedFormConfig {
            limit: 16384,
            error_handler: None,
        }
    }
}