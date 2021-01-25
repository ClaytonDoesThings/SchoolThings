use diesel::{
    prelude::*,
    PgConnection,
};

use rocket::{
    http::Cookies,
};

use super::{
    schema::sessions,
};

#[derive(Queryable)]
pub struct Session {
    pub id: i64,
    pub logged_in_user: Option<i64>,
}

#[derive(Insertable)]
#[table_name="sessions"]
pub struct NewSession {
    pub logged_in_user: i64,
}

impl NewSession {
    pub fn new(logged_in_user: i64) -> NewSession {
        NewSession {
            logged_in_user,
        }
    }
}

pub fn get_id_from_cookies(mut cookies: Cookies) -> Result<i64, String> {
    match cookies.get_private("session_id") {
        Some(session_id_cookie) => {
            match session_id_cookie.value().parse::<i64>() {
                Ok(session_id) => Ok(session_id),
                Err(e) => Err(format!("Failed to parse session_id: {}", e))
            }
        },
        None => Err("No session_id in cookies".to_string())
    }
}

pub fn get(pg_conn: &PgConnection, session_id: i64) -> Result<Session, String> {
    match sessions::table.find(session_id).first::<Session>(pg_conn) {
        Ok(session) => Ok(session),
        Err(e) => Err(format!("Failed to get session {}: {}", session_id, e))
    }
}

pub fn get_from_cookies(pg_conn: &PgConnection, cookies: Cookies) -> Result<Session, String> {
    match get_id_from_cookies(cookies) {
        Ok(session_id) => get(pg_conn, session_id),
        Err(e) => Err(format!("Failed to get session id: {}", e))
    }
}