use actix_web::{web, HttpMessage, FromRequest, HttpRequest};
use actix_web::dev::{ServiceRequest, Payload};
use actix_web::error::BlockingError;
use actix_web_httpauth::extractors::basic::BasicAuth;
use diesel::prelude::*;
use futures::future::{IntoFuture, Future};
use crate::state::AppState;
use crate::models::{User, Session};
use crate::ext::actix_web::{ConnectionInfoExt, HeaderMapExt};
use crate::api::errors::APIError;

pub fn validator(req: ServiceRequest, cred: BasicAuth) -> impl IntoFuture<Item = ServiceRequest, Error = actix_web::Error> {

    let state = req.app_data::<AppState>().expect("AppState missing");
    let ip_addr = req.connection_info().ip_address().unwrap();
    let ip_bin = bincode::serialize(&ip_addr).unwrap();
    let ua_opt = req.headers().user_agent();

    web::block(move || {

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

        //To improve performance we could cache the result in Redis
        let db = state.new_connection();
        let result: (Session, User) = match S::sessions.filter(
            S::user_id.eq(user).and(S::token.eq(pass))
        ).inner_join(U::users).first::<(Session, User)>(&db) {
            Ok(r) => r,
            Err(diesel::result::Error::NotFound) => return Err(APIError::IncorrectCredentials),
            Err(e) => return Err(e.into())
        };

        //This could also be moved onto a background threadpool
        diesel::update(&result.0).set((
            S::user_agent.eq(user_agent),
            S::last_ip.eq(ip_bin),
            S::last_used.eq(diesel::dsl::now),
        )).execute(&db)?;

        let authenticated_user = AuthenticatedUser {
            session: result.0,
            user: result.1,
        };

        Ok(authenticated_user)

    }).map(|result| {
        req.extensions_mut().insert(result);        //Move the user into the Request Extensions
        req
    }).map_err(|err: BlockingError<APIError>| {
        APIError::from(err).into()      //The signature ultimately requires an Actix Error
    })

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

    #[warn(unused_must_use)]
    pub fn assert_is_admin(&self) -> Result<(), APIError> {
        if self.user.is_admin {Ok(())} else {Err(APIError::Forbidden)}
    }

}

impl FromRequest for AuthenticatedUser {
    type Error = APIError;
    type Future = Result<Self, Self::Error>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        //Remove the User from the Request Extensions
        if let Some(user) = req.extensions_mut().remove::<AuthenticatedUser>() {
            Ok(user)
        } else {
            Err(APIError::InternalError("AuthenticatedUser not found in Request Extensions; use the Basic Authentication Middleware".to_owned()))
        }
    }
}
