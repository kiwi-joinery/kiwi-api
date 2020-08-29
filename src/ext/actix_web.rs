#![allow(non_snake_case)]

use actix_web::HttpResponse;
use actix_web::http::header;
use actix_web::http::header::HeaderMap;
use actix_web::dev::ConnectionInfo;
use std::net::{IpAddr, SocketAddr};


pub trait HttpResponseExt {
    //Redirect a route to a new location using HTTP Found
    fn found_to(destination: &str) -> HttpResponse;
}

impl HttpResponseExt for HttpResponse {

    fn found_to(destination: &str) -> HttpResponse {
        HttpResponse::Found().header(header::LOCATION, destination).finish()
    }

}


pub trait ConnectionInfoExt {
    fn ip_address(&self) -> Option<IpAddr>;
}

impl ConnectionInfoExt for ConnectionInfo {

    fn ip_address(&self) -> Option<IpAddr> {
        let socket_str = self.remote().map(|r| r.to_owned());
        let socket: Option<SocketAddr> = socket_str.and_then(|r| r.parse().ok());
        socket.map(|s| s.ip())
    }

}


pub trait HeaderMapExt {
    fn user_agent(&self) -> Option<String>;
}

impl HeaderMapExt for HeaderMap {

    fn user_agent(&self) -> Option<String> {
        self.get("User-Agent").and_then(|r| r.to_str().ok()).map(|r| r.to_owned())
    }

}