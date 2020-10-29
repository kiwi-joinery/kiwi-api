use crate::api::actix::{ConnectionInfoExt, HeaderMapExt};
use crate::api::errors::APIError;
use crate::models::{Session, User};
use crate::state::AppState;
use actix_web::dev::{Payload, ServiceRequest};
use actix_web::{web, FromRequest, HttpMessage, HttpRequest};
use actix_web_httpauth::extractors::basic::BasicAuth;
use diesel::prelude::*;
use futures::future::{err, ok, Ready};

pub async fn validator(
    req: ServiceRequest,
    cred: BasicAuth,
) -> Result<ServiceRequest, actix_web::Error> {
    let state = req.app_data::<AppState>().expect("AppState missing");
    let ip_addr = req.connection_info().ip_address();
    let ua_opt = req.headers().user_agent();

    let result = web::block(move || {
        let ip_addr = ip_addr?;
        let ip_bin = bincode::serialize(&ip_addr).unwrap();
        let user_agent = match ua_opt {
            Some(t) => t,
            None => return Err(APIError::BadAgent),
        };
        let user: i32 = match cred.user_id().parse() {
            Ok(val) => val,
            Err(_) => return Err(APIError::MissingCredentials),
        };
        let pass = match cred.password() {
            Some(val) => val,
            None => return Err(APIError::MissingCredentials),
        };

        use crate::schema::sessions::dsl as S;
        use crate::schema::users::dsl as U;

        let db = state.new_connection();
        let result: (Session, User) = match S::sessions
            .filter(S::user_id.eq(user).and(S::token.eq(pass)))
            .inner_join(U::users)
            .first::<(Session, User)>(&db)
        {
            Ok(r) => r,
            Err(diesel::result::Error::NotFound) => return Err(APIError::IncorrectCredentials),
            Err(e) => return Err(e.into()),
        };

        //TODO: this could be moved onto a background thread
        diesel::update(&result.0)
            .set((
                S::user_agent.eq(user_agent),
                S::last_ip.eq(ip_bin),
                S::last_used.eq(diesel::dsl::now),
            ))
            .execute(&db)?;

        Ok(AuthenticatedUser {
            session: result.0,
            user: result.1,
        })
    })
    .await;

    match result {
        Ok(result) => {
            req.extensions_mut().insert(result);
            Ok(req)
        }
        Err(err) => Err(APIError::from(err).into()),
    }
}

//AuthenticatedUser is an Extractor that can be used to access the user of an authenticated route
pub struct AuthenticatedUser {
    pub session: Session,
    pub user: User,
}

impl AuthenticatedUser {
    pub fn user_id(&self) -> i32 {
        self.session.user_id
    }
}

impl FromRequest for AuthenticatedUser {
    type Error = APIError;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        //Remove the User from the Request Extensions
        if let Some(user) = req.extensions_mut().remove::<AuthenticatedUser>() {
            ok(user)
        } else {
            err(APIError::InternalError("AuthenticatedUser not found in Request Extensions; use the Basic Authentication Middleware".to_owned()))
        }
    }
}
