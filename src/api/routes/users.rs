use crate::api::auth::AuthenticatedUser;
use crate::api::errors::APIError;
use crate::api::ok_json;
use crate::api::routes::password_reset::send_reset_email;
use crate::api::routes::session::AUTH_TOKEN_BYTES;
use crate::api::token::generate_token;
use crate::ext::postgres::functions::strpos;
use crate::ext::postgres::limit::{CountedLimitResult, CountingLimit};
use crate::models::{NewUser, User};
use crate::schema::users::dsl as U;
use crate::state::AppState;
use crate::state::Connection;
use actix_validated_forms::form::ValidatedForm;
use actix_validated_forms::query::ValidatedQuery;
use actix_web::web::{Data, Path};
use actix_web::{web, HttpResponse};
use diesel::prelude::*;
use futures::TryFutureExt;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(default)]
pub struct ListUserQuery {
    #[validate(range(min = 1, max = 100))]
    limit: i64,
    offset: i64,
    search: Option<String>,
}

impl Default for ListUserQuery {
    fn default() -> Self {
        ListUserQuery {
            limit: 20,
            offset: 0,
            search: None,
        }
    }
}

#[derive(Serialize)]
pub struct UserResponseItem {
    id: i32,
    name: String,
    email: String,
}

impl From<User> for UserResponseItem {
    fn from(u: User) -> Self {
        UserResponseItem {
            id: u.id,
            name: u.name,
            email: u.email,
        }
    }
}

pub async fn list(
    _auth: AuthenticatedUser,
    query: ValidatedQuery<ListUserQuery>,
    state: Data<AppState>,
) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<_, APIError> {
        let db = state.new_connection();

        let result: CountedLimitResult<User> = match &query.search {
            None => U::users
                .counted_limit(query.limit)
                .offset(query.offset)
                .load_with_total::<User>(&db)?,
            Some(search) => {
                let like = format!("%{}%", search);
                U::users
                    .filter(U::name.like(&like))
                    .or_filter(U::email.like(&like))
                    .order(strpos(U::name, search).asc())
                    .then_order_by(strpos(U::email, search).asc())
                    .counted_limit(query.limit)
                    .offset(query.offset)
                    .load_with_total::<User>(&db)?
            }
        };

        Ok(result.map(UserResponseItem::from))
    })
    .map_ok(ok_json)
    .err_into()
    .await
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserForm {
    #[validate(length(min = 1, max = 255))]
    name: String,
    #[validate(email)]
    email: String,
}

fn assert_email_available(db: &Connection, email: &String) -> Result<(), APIError> {
    let count = U::users
        .filter(U::email.eq(email))
        .count()
        .get_result::<i64>(db)?;
    if count > 0 {
        Err(APIError::BadRequest {
            code: "EMAIL_TAKEN".to_owned(),
            description: None,
        })
    } else {
        Ok(())
    }
}

pub async fn create(
    _auth: AuthenticatedUser,
    form: ValidatedForm<CreateUserForm>,
    state: Data<AppState>,
) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<UserResponseItem, APIError> {
        let db = state.new_connection();
        assert_email_available(&db, &form.email)?;

        let reset = generate_token(AUTH_TOKEN_BYTES);
        let insert = NewUser {
            name: form.name.clone(),
            email: form.email.clone(),
            password_hash: None,
            password_reset_token: Some(reset.clone()),
        };
        let user: User = diesel::insert_into(U::users)
            .values(&insert)
            .get_result(&db)?;

        match send_reset_email(&state.settings, &user.email, &reset) {
            Ok(_) => {}
            Err(e) => log::warn!("Unable to send reset email on account creation: {}", e),
        }

        Ok(user.into())
    })
    .map_ok(ok_json)
    .err_into()
    .await
}

fn resolve_user(
    auth: &AuthenticatedUser,
    user_id: i32,
    conn: &Connection,
) -> Result<User, APIError> {
    //If this isn't the logged in user - check that it exists and for admin
    if user_id == auth.user_id() {
        Ok(auth.user.clone())
    } else {
        let user = U::users.find(user_id).get_result::<User>(conn)?;
        Ok(user)
    }
}

pub async fn get(
    auth: AuthenticatedUser,
    user_id: Path<i32>,
    state: Data<AppState>,
) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<UserResponseItem, APIError> {
        let db = state.new_connection();
        let user = resolve_user(&auth, user_id.into_inner(), &db)?;
        Ok(user.into())
    })
    .map_ok(ok_json)
    .err_into()
    .await
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserForm {
    #[validate(length(min = 1, max = 255))]
    name: Option<String>,
    #[validate(email)]
    email: Option<String>,
}

pub async fn update(
    auth: AuthenticatedUser,
    user_id: Path<i32>,
    form: ValidatedForm<UpdateUserForm>,
    state: Data<AppState>,
) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<UserResponseItem, APIError> {
        let db = state.new_connection();
        let user_id = user_id.into_inner();
        let mut user = resolve_user(&auth, user_id, &db)?;

        match &form.name {
            Some(n) => {
                user.name = n.to_owned();
            }
            _ => {}
        }
        match &form.email {
            Some(e) => {
                if e != &user.email {
                    assert_email_available(&db, &e)?;
                    user.email = e.to_owned();
                }
            }
            _ => {}
        }

        diesel::update(&user).set(&user).execute(&db)?;

        Ok(user.into())
    })
    .map_ok(ok_json)
    .err_into()
    .await
}

pub async fn delete(
    auth: AuthenticatedUser,
    user_id: Path<i32>,
    state: Data<AppState>,
) -> Result<HttpResponse, APIError> {
    web::block(move || -> Result<(), APIError> {
        let db = state.new_connection();
        let user = resolve_user(&auth, user_id.into_inner(), &db)?;
        let user_id = user.id;

        use crate::schema::sessions::dsl as S;
        diesel::delete(S::sessions.filter(S::user_id.eq(user_id))).execute(&db)?;
        diesel::delete(U::users.filter(U::id.eq(user_id))).execute(&db)?;

        Ok(())
    })
    .map_ok(ok_json)
    .err_into()
    .await
}
