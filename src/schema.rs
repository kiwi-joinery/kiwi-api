table! {
    files (id) {
        id -> Int4,
    }
}

table! {
    sessions (id) {
        id -> Int4,
        user_id -> Int4,
        token -> Varchar,
        created -> Timestamptz,
        last_used -> Timestamptz,
        last_ip -> Bytea,
        user_agent -> Varchar,
    }
}

table! {
    users (id) {
        id -> Int4,
        name -> Varchar,
        email -> Varchar,
        password_hash -> Nullable<Varchar>,
        password_reset_token -> Nullable<Varchar>,
    }
}

joinable!(sessions -> users (user_id));

allow_tables_to_appear_in_same_query!(files, sessions, users,);
