use actix_web::error::ResponseError;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use serde::Serialize;

#[derive(Serialize)]
struct APIErrorResponse {
    code: String,
    description: Option<String>,
}

impl APIErrorResponse {
    pub fn new(code: String, description: Option<String>) -> APIErrorResponse {
        APIErrorResponse { code, description }
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
    InternalError(String),
    NotImplemented,
}

impl APIError {
    //Map APIErrors to a HTTP Status Code
    fn status_code(&self) -> StatusCode {
        match self {
            APIError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            APIError::BadAgent { .. } => StatusCode::BAD_REQUEST,
            APIError::ValidationError(_) => StatusCode::BAD_REQUEST,
            APIError::MissingCredentials => StatusCode::UNAUTHORIZED,
            APIError::IncorrectCredentials => StatusCode::UNAUTHORIZED,
            APIError::Forbidden => StatusCode::FORBIDDEN,
            APIError::NotFound => StatusCode::NOT_FOUND,
            APIError::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            APIError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            APIError::NotImplemented => StatusCode::NOT_IMPLEMENTED,
        }
    }

    //Create an APIErrorResponse from an APIError
    fn transform(&self) -> APIErrorResponse {
        match self {
            APIError::BadRequest { code, description } => {
                APIErrorResponse::new(code.to_owned(), description.to_owned())
            }
            APIError::BadAgent => APIErrorResponse::new("BAD_AGENT".to_owned(), None),
            APIError::ValidationError(s) => {
                APIErrorResponse::new("VALIDATION_ERROR".to_owned(), Some(s.to_owned()))
            }
            APIError::MissingCredentials => {
                APIErrorResponse::new("MISSING_CREDENTIALS".to_owned(), None)
            }
            APIError::IncorrectCredentials => {
                APIErrorResponse::new("INCORRECT_CREDENTIALS".to_owned(), None)
            }
            APIError::Forbidden => APIErrorResponse::new("FORBIDDEN".to_owned(), None),
            APIError::NotFound => APIErrorResponse::new("NOT_FOUND".to_owned(), None),
            APIError::MethodNotAllowed => {
                APIErrorResponse::new("METHOD_NOT_ALLOWED".to_owned(), None)
            }
            APIError::InternalError(e) => {
                //Internal server errors are only shown when debug is enabled, for security reasons
                let hidden = if cfg!(debug_assertions) {
                    Some(e.to_owned())
                } else {
                    None
                };
                APIErrorResponse::new("INTERNAL_SERVER_ERROR".to_owned(), hidden)
            }
            APIError::NotImplemented => APIErrorResponse::new("NOT_IMPLEMENTED".to_owned(), None),
        }
    }
}

impl std::fmt::Display for APIError {
    fn fmt(&self, _f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        unimplemented!("Use error_response()")
    }
}

impl ResponseError for APIError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(self.transform())
    }
}

//Note that not finding an expected row will be transformed into an APIError::NotFound
impl From<diesel::result::Error> for APIError {
    fn from(err: diesel::result::Error) -> Self {
        use diesel::result::Error;
        match err {
            Error::NotFound => APIError::NotFound,
            err => APIError::InternalError(format!("{}", err)),
        }
    }
}

impl<T: std::fmt::Debug + Into<APIError>> From<actix_web::error::BlockingError<T>> for APIError {
    fn from(err: actix_web::error::BlockingError<T>) -> Self {
        use actix_web::error::BlockingError;
        match err {
            BlockingError::Error(e) => e.into(),
            BlockingError::Canceled => {
                APIError::InternalError("Blocking operation execution error".to_owned())
            }
        }
    }
}

impl From<actix_web::error::PathError> for APIError {
    fn from(_err: actix_web::error::PathError) -> Self {
        APIError::NotFound
    }
}

impl From<actix_web::error::UrlencodedError> for APIError {
    fn from(err: actix_web::error::UrlencodedError) -> Self {
        APIError::ValidationError(format!("{}", err))
    }
}

impl From<actix_web::error::QueryPayloadError> for APIError {
    fn from(err: actix_web::error::QueryPayloadError) -> Self {
        APIError::ValidationError(format!("{}", err))
    }
}

impl From<actix_validated_forms::validator::ValidationErrors> for APIError {
    fn from(err: actix_validated_forms::validator::ValidationErrors) -> Self {
        APIError::ValidationError(format!("{}", err))
    }
}

impl<T> From<actix_validated_forms::error::ValidatedFormError<T>> for APIError
where
    T: std::fmt::Debug + std::fmt::Display + Into<APIError>,
{
    fn from(e: actix_validated_forms::error::ValidatedFormError<T>) -> Self {
        use actix_validated_forms::error::ValidatedFormError;
        match e {
            ValidatedFormError::Deserialization(u) => u.into(),
            ValidatedFormError::Validation(v) => v.into(),
        }
    }
}

impl From<bcrypt::BcryptError> for APIError {
    fn from(err: bcrypt::BcryptError) -> Self {
        APIError::InternalError(format!("{}", err))
    }
}
