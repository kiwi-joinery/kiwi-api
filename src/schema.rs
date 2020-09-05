table! {
    files (id) {
        id -> Int4,
        bytes -> Int8,
        extension -> Nullable<Varchar>,
    }
}

table! {
    gallery_files (item_id, file_id) {
        item_id -> Int4,
        file_id -> Int4,
        height -> Int4,
        width -> Int4,
    }
}

table! {
    gallery_items (id) {
        id -> Int4,
        description -> Nullable<Varchar>,
        original_file_id -> Int4,
        position -> Varchar,
        category -> Varchar,
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

joinable!(gallery_files -> files (file_id));
joinable!(gallery_files -> gallery_items (item_id));
joinable!(gallery_items -> files (original_file_id));
joinable!(sessions -> users (user_id));

allow_tables_to_appear_in_same_query!(
    files,
    gallery_files,
    gallery_items,
    sessions,
    users,
);
