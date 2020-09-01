use actix_web::dev::HttpResponseBuilder;
use actix_web::HttpResponse;
use serde::Serialize;

#[derive(Serialize, Debug)]
struct Response<D: Serialize, E: Serialize> {
    response: &'static str,
    data: Option<D>,
    error: Option<E>,
}

pub trait APIResponse {
    fn success<T: Serialize>(&mut self, data: T) -> HttpResponse;

    fn error<T: Serialize>(&mut self, error: T) -> HttpResponse;
}

impl APIResponse for HttpResponseBuilder {
    fn success<T: Serialize>(&mut self, data: T) -> HttpResponse {
        let x: Response<T, ()> = Response {
            response: "SUCCESS",
            data: Some(data),
            error: None,
        };
        self.json(x)
    }

    fn error<T: Serialize>(&mut self, error: T) -> HttpResponse {
        let x: Response<(), T> = Response {
            response: "ERROR",
            data: None,
            error: Some(error),
        };
        self.json(x)
    }
}

pub fn ok_response<T: Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok().success(data)
}

#[derive(Debug, Serialize)]
pub struct CountedLimitResult<T> {
    pub results: Vec<T>,
    pub total: i64,
}
