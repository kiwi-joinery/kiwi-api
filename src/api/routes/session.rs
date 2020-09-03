use crate::api::auth::AuthenticatedUser;
use crate::api::errors::APIError;
use crate::api::ok_json;
use crate::api::routes::users::UserResponseItem;
use crate::api::token::generate_token;
use crate::ext::actix::{ConnectionInfoExt, HeaderMapExt};
use crate::models::{NewSession, Session, User};
use crate::schema::sessions::dsl as S;
use crate::schema::users::dsl as U;
use crate::state::AppState;
use actix_web::web::{Data, Form, Path};
use actix_web::{web, HttpRequest, HttpResponse};
use bcrypt::verify;
use diesel::prelude::*;
use futures::TryFutureExt;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

// Use a 16 byte / 128 bit token
// https://github.com/OWASP/CheatSheetSeries/blob/master/cheatsheets/Session_Management_Cheat_Sheet.md#session-id-length
pub const SESSION_TOKEN_BYTES: u8 = 32;

#[derive(Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    token: String,
    user: UserResponseItem,
}

pub async fn password_login(
    form: Form<LoginForm>,
    state: Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, APIError> {
    let ip_addr = req.connection_info().ip_address().unwrap();
    let ip_bin = bincode::serialize(&ip_addr).unwrap();
    let ua_opt = req.headers().user_agent();

    web::block(move || {
        let db = state.new_connection();

        let user_agent = match ua_opt {
            Some(t) => t,
            None => return Err(APIError::BadAgent),
        };

        //Fetch the user with the submitted email
        let user: User = match U::users.filter(U::email.eq(&form.email)).first::<User>(&db) {
            Ok(r) => r,
            Err(diesel::result::Error::NotFound) => return Err(APIError::IncorrectCredentials),
            Err(e) => return Err(e.into()),
        };

        //Check that the user does actually have a password set
        let hashed = match &user.password_hash {
            Some(val) => val.clone(),
            None => return Err(APIError::IncorrectCredentials),
        };

        use std::time::Instant;
        let start = Instant::now();
        //Check that the password matches
        if !(verify(&form.password, &hashed)?) {
            return Err(APIError::IncorrectCredentials);
        };
        println!("{}ms to verify password ", start.elapsed().as_millis());

        //See if a session exists for this IP + Agent
        let session: Option<Session> = Session::belonging_to(&user)
            .filter(
                S::last_ip
                    .eq(ip_bin.clone())
                    .and(S::user_agent.eq(user_agent.clone())),
            )
            .first::<Session>(&db)
            .optional()?;

        let token = match session {
            Some(val) => val.token,
            None => {
                //If not create new token and session
                let new_token = generate_token(SESSION_TOKEN_BYTES);
                let session = NewSession {
                    user_id: user.id,
                    token: new_token.clone(),
                    last_ip: ip_bin,
                    user_agent,
                };
                diesel::insert_into(S::sessions)
                    .values(&session)
                    .execute(&db)?;
                new_token
            }
        };

        Ok(LoginResponse {
            token,
            user: user.into(),
        })
    })
    .map_ok(ok_json)
    .err_into()
    .await
}

//Deletes the currently authenticated session
pub async fn logout(
    auth: AuthenticatedUser,
    state: Data<AppState>,
) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<_, APIError> {
        let db = state.new_connection();
        diesel::delete(&auth.session).execute(&db)?;
        Ok(())
    })
    .map_ok(ok_json)
    .err_into()
    .await
}

#[derive(Serialize)]
struct SessionResponseItem {
    id: i32,
    created: i64,
    last_used: i64,
    last_ip: Option<String>,
    user_agent: String,
    is_current: bool,
}

fn ip_bytes_to_str(ip_bytes: Vec<u8>) -> Option<String> {
    bincode::deserialize::<IpAddr>(&ip_bytes)
        .ok()
        .map(|ip| ip.to_string())
}

pub async fn list(
    auth: AuthenticatedUser,
    state: Data<AppState>,
) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<_, APIError> {
        let db = state.new_connection();
        let sessions: Vec<Session> = Session::belonging_to(&auth.user).load::<Session>(&db)?;
        let formatted = sessions
            .into_iter()
            .map(|s| SessionResponseItem {
                id: s.id,
                created: s.created.timestamp(),
                last_used: s.last_used.timestamp(),
                last_ip: ip_bytes_to_str(s.last_ip),
                user_agent: s.user_agent,
                is_current: (s.id == auth.session.id),
            })
            .collect::<Vec<_>>();
        Ok(formatted)
    })
    .map_ok(ok_json)
    .err_into()
    .await
}

pub async fn delete(
    auth: AuthenticatedUser,
    session_id: Path<i32>,
    state: Data<AppState>,
) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<_, APIError> {
        let db = state.new_connection();

        let session: Session = Session::belonging_to(&auth.user)
            .filter(S::id.eq(session_id.into_inner()))
            .first::<Session>(&db)?;

        diesel::delete(&session).execute(&db)?;
        Ok(())
    })
    .map_ok(ok_json)
    .err_into()
    .await
}
