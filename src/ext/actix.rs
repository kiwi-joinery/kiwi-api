use actix_web::dev::ConnectionInfo;
use actix_web::http::header::HeaderMap;
use std::net::{IpAddr, SocketAddr};

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
        self.get("User-Agent")
            .and_then(|r| r.to_str().ok())
            .map(|r| r.to_owned())
    }
}
