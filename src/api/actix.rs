use crate::api::errors::APIError;
use actix_web::dev::ConnectionInfo;
use actix_web::http::header::HeaderMap;
use std::net::{IpAddr, SocketAddr};

pub trait ConnectionInfoExt {
    fn ip_address(&self) -> Result<IpAddr, APIError>;
}

impl ConnectionInfoExt for ConnectionInfo {
    fn ip_address(&self) -> Result<IpAddr, APIError> {
        let str = match self.remote() {
            None => return Err(APIError::InternalError("remote() was none".to_string())),
            Some(x) => x,
        };
        match str.parse::<SocketAddr>() {
            Ok(x) => return Ok(x.ip()),
            _ => {}
        };
        match str.parse::<IpAddr>() {
            Ok(x) => return Ok(x),
            _ => {}
        };
        Err(APIError::InternalError(format!("Couldn't parse {}", str)))
    }
}

pub trait HeaderMapExt {
    fn user_agent(&self) -> Option<String>;
}

impl HeaderMapExt for HeaderMap {
    fn user_agent(&self) -> Option<String> {
        self.get("User-Agent")
            .and_then(|r| r.to_str().ok())
            .map(|r| r.to_owned())
    }
}
