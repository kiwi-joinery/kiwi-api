use actix_web::{web, HttpResponse};
use actix_web::web::{Data, Path};
use futures::future::Future;
use crate::state::AppState;
use crate::api::errors::APIError;
use crate::api::response::ok_response;
use crate::api::auth::AuthenticatedUser;
use diesel::prelude::*;
use bcrypt::{hash, DEFAULT_COST};
use serde::{Serialize, Deserialize};
use crate::models::{User, NewUser};
use crate::schema::users::dsl as U;
use validator::Validate;
use crate::ext::postgres::functions::strpos;
use crate::api::token::generate_token;
use crate::state::Connection;
use crate::api::routes::password_reset::RESET_TOKEN_BYTES;
use crate::ext::validation::form::ValidatedForm;
use crate::ext::validation::query::ValidatedQuery;
use crate::ext::postgres::limit::{CountingLimit, CountedLimitResult};

#[derive(Debug, Deserialize, Validate)]
#[serde(default)]
pub struct ListUserQuery {
    #[validate(range(min="1", max="100"))]
    limit: i64,
    offset: i64,
    search: Option<String>,
}

impl Default for ListUserQuery {
    fn default() -> Self {
        ListUserQuery { limit: 20, offset: 0, search: None }
    }
}

#[derive(Serialize)]
pub struct UserResponseItem {
    id: i32,
    name: String,
    email: String,
    is_admin: bool,
}

impl From<User> for UserResponseItem {
    fn from(u: User) -> Self {
        UserResponseItem {
            id: u.id,
            name: u.name,
            email: u.email,
            is_admin: u.is_admin
        }
    }
}

pub fn list(auth: AuthenticatedUser, query: ValidatedQuery<ListUserQuery>, state: Data<AppState>) -> impl Future<Item = HttpResponse, Error = APIError> {
    web::block(move || -> Result<_, APIError> {
        let db = state.new_connection();
        auth.assert_is_admin()?;        //Admin required to list users

        let result: (CountedLimitResult<User>) = match &query.search {
            None => {
                U::users.counted_limit(query.limit).offset(query.offset).load_with_total::<User>(&db)?
            },
            Some(search) => {
                let like = format!("%{}%", search);
                U::users.filter(U::name.like(&like))
                    .or_filter(U::email.like(&like))
                    .order(strpos(U::name, search).asc())
                    .then_order_by(strpos(U::email, search).asc())
                    .counted_limit(query.limit).offset(query.offset)
                    .load_with_total::<User>(&db)?
            },
        };

        Ok(result.map(UserResponseItem::from))

    }).map(ok_response).from_err()
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserForm {
    #[validate(length(min="1", max="255"))]
    name: String,
    #[validate(email)]
    email: String,
    is_admin: bool,
}

fn assert_email_available(db: &Connection, email: &String) -> Result<(), APIError> {
    let count = U::users.filter(U::email.eq(email)).count().get_result::<i64>(db)?;
    if count > 0 { Err(APIError::BadRequest { code: "EMAIL_TAKEN".to_owned(), description: None }) } else { Ok(()) }
}

pub fn create(auth: AuthenticatedUser, form: ValidatedForm<CreateUserForm>, state: Data<AppState>) -> impl Future<Item = HttpResponse, Error = APIError> {
    web::block(move || -> Result<UserResponseItem, APIError> {
        let db = state.new_connection();
        auth.assert_is_admin()?;        //Admin required to create users
        assert_email_available(&db, &form.email)?;

        let reset = generate_token(RESET_TOKEN_BYTES);

        let insert = NewUser {
            name: form.name.clone(),
            email: form.email.clone(),
            password_hash: None,
            is_admin: form.is_admin,
            password_reset_token: Some(reset),
        };

        let user: User = diesel::insert_into(U::users).values(&insert).get_result(&db)?;

        if insert.password_reset_token.is_some() {
            //TODO: Email
            println!("Send email");
        }

        Ok(user.into())
    }).map(ok_response).from_err()
}

fn resolve_user(auth: &AuthenticatedUser, user_id: i32, conn: &Connection) -> Result<User, APIError> {
    //If this isn't the logged in user - check that it exists and for admin
    if user_id == auth.user_id() {
        Ok(auth.user.clone())
    } else {
        let user = U::users.find(user_id).get_result::<User>(conn)?;
        auth.assert_is_admin()?;
        Ok(user)
    }
}

pub fn get(auth: AuthenticatedUser, user_id: Path<i32>, state: Data<AppState>) -> impl Future<Item = HttpResponse, Error = APIError> {
    web::block(move || -> Result<UserResponseItem, APIError> {
        let db = state.new_connection();
        let user = resolve_user(&auth, user_id.into_inner(), &db)?;
        Ok(user.into())
    }).map(ok_response).from_err()
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserForm {
    #[validate(length(min="1", max="255"))]
    name: Option<String>,
    #[validate(email)]
    email: Option<String>,
    password: Option<String>,
    is_admin: Option<bool>,
}

pub fn update(auth: AuthenticatedUser, user_id: Path<i32>, form: ValidatedForm<UpdateUserForm>, state: Data<AppState>) -> impl Future<Item = HttpResponse, Error = APIError> {
    web::block(move || -> Result<UserResponseItem, APIError> {
        let db = state.new_connection();
        let user_id = user_id.into_inner();
        let mut user = resolve_user(&auth, user_id, &db)?;

        match &form.name {
            Some(n) => {user.name = n.to_owned();}
            _ => {}
        }
        match &form.email {
            Some(e) => if e != &user.email {
                assert_email_available(&db, &e)?;
                user.email = e.to_owned();
            },
            _ => {}
        }
        match &form.password {
            Some(p) => {
                //Only users may do this - admins use reset link instead
                if user_id != auth.user_id() {return Err(APIError::Forbidden)}
                let hashed = hash(p, DEFAULT_COST)?;
                user.password_hash = Some(hashed);
            },
            _ => {}
        };
        match form.is_admin {
            Some(a) => {
                //Only admins may change permissions
                auth.assert_is_admin()?;
                user.is_admin = a;
            },
            _ => {}
        }
        diesel::update(&user).set(&user).execute(&db)?;

        Ok(user.into())
    }).map(ok_response).from_err()
}

pub fn delete(auth: AuthenticatedUser, user_id: Path<i32>, state: Data<AppState>) -> impl Future<Item = HttpResponse, Error = APIError> {
    web::block(move || -> Result<(), APIError> {
        let db = state.new_connection();
        let user = resolve_user(&auth, user_id.into_inner(), &db)?;
        let user_id = user.id;

        use crate::schema::lectures::dsl as L;
        use crate::schema::sessions::dsl as S;

        diesel::delete(L::lectures.filter(L::user_id.eq(user_id))).execute(&db)?;
        diesel::delete(S::sessions.filter(S::user_id.eq(user_id))).execute(&db)?;
        diesel::delete(U::users.filter(U::id.eq(user_id))).execute(&db)?;

        Ok(())
    }).map(ok_response).from_err()
}
