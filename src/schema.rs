table! {
    sessions (id) {
        id -> Int8,
        logged_in_user -> Nullable<Int8>,
    }
}

table! {
    users (id) {
        id -> Int8,
        username -> Varchar,
        email -> Varchar,
        password_hash -> Bpchar,
    }
}

allow_tables_to_appear_in_same_query!(
    sessions,
    users,
);
