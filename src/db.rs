use diesel::{
    prelude::*,
    PgConnection,
};

use rocket::{
    http::Cookies,
};

use super::{
    models,
    schema,
};

pub fn get_session_id(mut cookies: Cookies) -> Result<i64, String> {
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

pub fn get_session(pg_conn: &PgConnection, session_id: i64) -> Result<models::Session, String> {
    match schema::sessions::table.find(session_id).first::<models::Session>(pg_conn) {
        Ok(session) => Ok(session),
        Err(e) => Err(format!("Failed to get session {}: {}", session_id, e))
    }
}

pub fn get_session_from_cookies(pg_conn: &PgConnection, cookies: Cookies) -> Result<models::Session, String> {
    match get_session_id(cookies) {
        Ok(session_id) => get_session(pg_conn, session_id),
        Err(e) => Err(format!("Failed to get session id: {}", e))
    }
}

pub fn get_user(pg_conn: &PgConnection, user_id: i64) -> Result<models::User, String> {
    match schema::users::table.find(user_id).first::<models::User>(pg_conn) {
        Ok(user) => Ok(user),
        Err(e) => Err(format!("Failed to get user: {}", e))
    }
}

pub fn get_user_by_username(pg_conn: &PgConnection, username: String) -> Result<models::User, String> {
    match schema::users::table.filter(schema::users::username.eq(username)).first::<models::User>(pg_conn) {
        Ok(user) => Ok(user),
        Err(e) => Err(format!("Failed to get user by username {}", e))
    }
}

pub fn get_user_from_session(pg_conn: &PgConnection, session: &models::Session) -> Result<models::User, String> {
    match session.logged_in_user {
        Some(logged_in_user) => get_user(pg_conn, logged_in_user),
        None => Err("Session is not logged in".to_string())
    }
}

pub fn get_user_from_cookies(pg_conn: &PgConnection, cookies: Cookies) -> Result<models::User, String> {
    match get_session_from_cookies(pg_conn, cookies) {
        Ok(session) => {
            match session.logged_in_user {
                Some(logged_in_user) => get_user(pg_conn, logged_in_user),
                None => Err("Session is not logged in".to_string())
            }
        },
        Err(_) => Err("Failed to get session from cookies".to_string())
    }
}

pub fn get_app_by_title(pg_conn: &PgConnection, title: &str) -> Result<models::App, String> {
    match schema::apps::table.filter(
        schema::apps::title.eq(title)
    ).first::<models::App>(pg_conn) {
        Ok(app) => Ok(app),
        Err(e) => Err(format!("Failed to get app by title {}", e))
    }
}

pub fn get_apps(pg_conn: &PgConnection) -> Result<Vec<models::App>, String> {
    match schema::apps::table.load::<models::App>(pg_conn) {
        Ok(apps) => Ok(apps),
        Err(e) => Err(format!("Failed to get apps {}", e))
    }
}