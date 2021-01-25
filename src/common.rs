use ammonia::clean_text;
use diesel::PgConnection;

use percent_encoding::{
    NON_ALPHANUMERIC,
    percent_encode,
};

use regex::Regex;

use rocket::http::Cookies;
use rocket_contrib::templates::tera::Context;

use serde::Serialize;

use super::{
    sessions,
    users,
};

pub fn default_context() -> Context {
    let mut context = Context::new();
    context.insert("domain", "https://schoolthings.xyz");
    return context;
}

pub fn signed_in_context(pg_conn: &PgConnection, cookies: Cookies) -> (Context, Option<sessions::Session>, Option<users::User>) {
    let mut context = default_context();
    match sessions::get_from_cookies(&pg_conn, cookies) {
        Ok(session) => {
            match users::get_from_session(&pg_conn, &session) {
                Ok(user) => {
                    context.insert("user", &user);
                    context.insert("clean_user", &users::CleanUser::from_user(&user));
                    (context, Some(session), Some(user))
                },
                Err(_) => (context, Some(session), None)
            }
        },
        Err(_) => (context, None, None)
    }
}

pub fn validate_title(title: &str) -> bool {
    Regex::new(r"^[0-9A-Za-z][0-9A-Za-z_-]{1,}[0-9A-Za-z]$").unwrap().is_match(title)
}

#[derive(Serialize)]
pub struct Cleaned {
    pub url: String,
    pub html: String
}

impl Cleaned {
    pub fn new(string: &String) -> Cleaned {
        Cleaned {
            url: percent_encode(string.as_bytes(), NON_ALPHANUMERIC).to_string(),
            html: clean_text(&string)
        }
    }
}