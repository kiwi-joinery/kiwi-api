use actix_web::HttpResponse;
use actix_web::error::ResponseError;
use actix_web::http::StatusCode;
use super::response::APIResponse;
use serde::Serialize;

#[derive(Serialize)]
struct APIErrorResponse {
    code: String,
    description: Option<String>,
}

impl APIErrorResponse {

    pub fn code(code: String) -> APIErrorResponse {
        APIErrorResponse {code: code.to_owned(), description: None}
    }

    pub fn description(code: String, description: Option<String>) -> APIErrorResponse {
        APIErrorResponse {code, description}
    }

    //Internal server errors are only shown when debug is enabled, for security reasons
    pub fn internal(source: String, cause: String) -> APIErrorResponse {
        let description = format!("{}: {}", source, cause);
        let hidden = if cfg!(debug_assertions) {Some(description)} else {None};
        APIErrorResponse {code: "INTERNAL_SERVER_ERROR".to_owned(), description: hidden}
    }

}

#[derive(Debug)]
pub enum APIError {
    BadRequest {
        code: String,
        description: Option<String>,
    },
    BadAgent,
    ValidationError(String),
    MissingCredentials,
    IncorrectCredentials,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    UnderlyingError(String, failure::Error),
    InternalError(String),
    NotImplemented,
}

impl APIError {

    //Map APIErrors to a HTTP Status Code
    fn status_code(&self) -> StatusCode {
        match self {
            APIError::BadRequest {..} => StatusCode::BAD_REQUEST,
            APIError::BadAgent {..} => StatusCode::BAD_REQUEST,
            APIError::ValidationError(_) => StatusCode::BAD_REQUEST,
            APIError::MissingCredentials => StatusCode::UNAUTHORIZED,
            APIError::IncorrectCredentials => StatusCode::UNAUTHORIZED,
            APIError::Forbidden => StatusCode::FORBIDDEN,
            APIError::NotFound => StatusCode::NOT_FOUND,
            APIError::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            APIError::UnderlyingError(_,_) => StatusCode::INTERNAL_SERVER_ERROR,
            APIError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            APIError::NotImplemented => StatusCode::NOT_IMPLEMENTED,
        }
    }

    //Create an APIErrorResponse from an APIError
    fn transform(&self) -> APIErrorResponse {
        match self {
            APIError::BadRequest {code, description} => APIErrorResponse::description(code.to_owned(), description.to_owned()),
            APIError::BadAgent => APIErrorResponse::description("BAD_AGENT".to_owned(), None),
            APIError::ValidationError(s) => APIErrorResponse::description("VALIDATION_ERROR".to_owned(), Some(s.to_owned())),
            APIError::MissingCredentials => APIErrorResponse::code("MISSING_CREDENTIALS".to_owned()),
            APIError::IncorrectCredentials => APIErrorResponse::code("INCORRECT_CREDENTIALS".to_owned()),
            APIError::Forbidden => APIErrorResponse::code("FORBIDDEN".to_owned()),
            APIError::NotFound => APIErrorResponse::code("NOT_FOUND".to_owned()),
            APIError::MethodNotAllowed => APIErrorResponse::code("METHOD_NOT_ALLOWED".to_owned()),
            APIError::UnderlyingError(s, e) => APIErrorResponse::internal(s.to_owned(), e.to_string()),
            APIError::InternalError(e) => APIErrorResponse::internal("App".to_owned(), e.to_owned()),
            APIError::NotImplemented => APIErrorResponse::code("NOT_IMPLEMENTED".to_owned()),
        }
    }
}

//Implementing ResponseError requires Display however we do not actually need it
impl std::fmt::Display for APIError {
    fn fmt(&self, _f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        unimplemented!("Use render_response()")
    }
}

impl ResponseError for APIError {

    fn error_response(&self) -> HttpResponse {
        unimplemented!("Use render_response()")
    }

    fn render_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).error(self.transform())
    }

}

//Note that not finding an expected row will be transformed into an APIError::NotFound
impl From<diesel::result::Error> for APIError {
    fn from(err: diesel::result::Error) -> Self {
        use diesel::result::Error;
        match err {
            Error::NotFound => APIError::NotFound,
            err => APIError::UnderlyingError("Diesel".to_owned(), err.into())
        }
    }
}

impl<T: std::fmt::Debug + Into<APIError>> From<actix_web::error::BlockingError<T>> for APIError {
    fn from(err: actix_web::error::BlockingError<T>) -> Self {
        use actix_web::error::BlockingError;
        match err {
            BlockingError::Error(e) => {e.into()},
            BlockingError::Canceled => APIError::InternalError("Blocking operation execution error".to_owned()),
        }
    }
}

impl From<actix_web::error::UrlencodedError> for APIError {
    fn from(err: actix_web::error::UrlencodedError) -> Self {
        APIError::ValidationError(format!("{}", err))
    }
}

impl From<actix_web::error::PathError> for APIError {
    fn from(_err: actix_web::error::PathError) -> Self {
        APIError::NotFound
    }
}

impl From<actix_web::error::QueryPayloadError> for APIError {
    fn from(err: actix_web::error::QueryPayloadError) -> Self {
        APIError::ValidationError(format!("{}", err))
    }
}

impl From<bcrypt::BcryptError> for APIError {
    fn from(err: bcrypt::BcryptError) -> Self {
        APIError::UnderlyingError("Bcrypt".to_owned(), err.into())
    }
}

impl From<validator::ValidationErrors> for APIError {
    fn from(err: validator::ValidationErrors) -> Self {
        APIError::ValidationError(format!("{}", err))
    }
}

impl From<crate::ext::validation::form::ValidatedFormError> for APIError {
    fn from(e: crate::ext::validation::form::ValidatedFormError) -> Self {
        use crate::ext::validation::form::ValidatedFormError;
        match e {
            ValidatedFormError::Deserialization(u) => {u.into()},
            ValidatedFormError::Validation(v) => {v.into()}
        }
    }
}

impl From<crate::ext::validation::query::ValidatedQueryError> for APIError {
    fn from(e: crate::ext::validation::query::ValidatedQueryError) -> Self {
        use crate::ext::validation::query::ValidatedQueryError;
        match e {
            ValidatedQueryError::Deserialization(u) => {u.into()},
            ValidatedQueryError::Validation(v) => {v.into()}
        }
    }
}