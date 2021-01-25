table! {
    apps (id) {
        id -> Int8,
        owner_id -> Int8,
        title -> Varchar,
        description -> Varchar,
        domain -> Varchar,
        token -> Bpchar,
        connected -> Bool,
        connected_error -> Varchar,
    }
}

table! {
    repos (id) {
        id -> Int8,
        owner_id -> Int8,
        title -> Varchar,
        description -> Varchar,
        apps -> Array<Int8>,
    }
}

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
    apps,
    repos,
    sessions,
    users,
);
