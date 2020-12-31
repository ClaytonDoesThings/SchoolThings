#![feature(proc_macro_hygiene, decl_macro)]

use ammonia::clean_text;
use regex::Regex;
use std::path::Path;

#[macro_use] extern crate rocket;
use rocket::{
    http::{
        Cookie,
        Cookies,
    },
    request::Form,
    response::{
        NamedFile,
        Redirect,
        status,
    },
    uri,
};

#[macro_use] extern crate rocket_contrib;
use rocket_contrib::templates::{
    Template,
    tera::Context,
};

#[macro_use] extern crate diesel;
use diesel::prelude::*;
use diesel::PgConnection;
pub mod schema;
pub mod models;
pub mod crypt_eq;
use crate::crypt_eq::CryptExpressionMethods;

#[database("postgres")]
struct DbConn(PgConnection);

fn default_context() -> Context {
    let mut context = Context::new();
    context.insert("domain", "https://schoolthings.xyz");
    return context;
}

fn signed_in_context(db_conn: DbConn, cookies: Cookies) -> Context {
    let mut context = default_context();
    match get_session_from_cookies(&*db_conn, cookies) {
        Ok(session) => {
            match get_user_from_session(&*db_conn, session) {
                Ok(user) => {
                    context.insert("user", &user);
                    context.insert("clean_user", &models::CleanUser::new(user));
                },
                Err(_e) => println!("{}", _e)
            }
        },
        Err(_e) => println!("{}", _e)
    }
    return context;
}

fn get_session_id(mut cookies: Cookies) -> Result<i64, String> {
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

fn get_session(pg_conn: &PgConnection, session_id: i64) -> Result<models::Session, String> {
    match schema::sessions::table.find(session_id).first::<models::Session>(pg_conn) {
        Ok(session) => Ok(session),
        Err(e) => Err(format!("Failed to get session {}: {}", session_id, e))
    }
}

fn get_session_from_cookies(pg_conn: &PgConnection, cookies: Cookies) -> Result<models::Session, String> {
    match get_session_id(cookies) {
        Ok(session_id) => get_session(pg_conn, session_id),
        Err(e) => Err(format!("Failed to get session id: {}", e))
    }
}

fn get_user(pg_conn: &PgConnection, user_id: i64) -> Result<models::User, String> {
    match schema::users::table.find(user_id).first::<models::User>(pg_conn) {
        Ok(user) => Ok(user),
        Err(e) => Err(format!("Failed to get user: {}", e))
    }
}

fn get_user_from_session(pg_conn: &PgConnection, session: models::Session) -> Result<models::User, String> {
    match session.logged_in_user {
        Some(logged_in_user) => get_user(pg_conn, logged_in_user),
        None => Err("Session is not logged in".to_string())
    }
}

#[get("/")]
fn home(db_conn: DbConn, cookies: Cookies) -> Template { Template::render("home", &signed_in_context(db_conn, cookies)) }

#[get("/favicon.ico")]
fn favicon() -> Result<NamedFile, status::NotFound<String>> {
    let path = Path::new("favicon.ico");
    NamedFile::open(&path).map_err(|e| status::NotFound(e.to_string()))
}

#[get("/sitemap.xml")]
fn sitemap() -> Template { Template::render("sitemap", &default_context()) }

#[get("/login?<error>")]
fn login(error: Option<String>, db_conn: DbConn, cookies: Cookies) -> Template {
    let mut context = signed_in_context(db_conn, cookies);
    match error {
        Some(error) => context.insert("error", &clean_text(&error)),
        None => {}
    }
    Template::render("login", &context)
}

#[get("/signup?<error>")]
fn signup(error: Option<String>, db_conn: DbConn, cookies: Cookies) -> Template {
    let mut context = signed_in_context(db_conn, cookies);
    match error {
        Some(error) => context.insert("error", &clean_text(&error)),
        None => {}
    }
    Template::render("signup", &context)
}

fn update_session_logged_in_user(user: models::User, mut cookies: Cookies, db_conn: DbConn) -> Result<String, String> {
    match cookies.get_private("session_id") {
        Some(session_id_cookie) => {
            match session_id_cookie.value().parse::<i64>() {
                Ok(session_id) => {
                    match diesel::update(schema::sessions::table.find(session_id)).set(schema::sessions::logged_in_user.eq(user.id)).execute(&*db_conn) {
                        Ok(_v) => Ok("Signed in!".to_string()),
                        Err(_e) => Err("Failed to update session".to_string())
                    }
                },
                Err(_e) => Err("Failed to parse session_id cookie".to_string())
            }
        },
        None => {
            match diesel::insert_into(schema::sessions::table).values(&models::NewSession::new(user.id)).get_result::<models::Session>(&*db_conn) {
                Ok(session) => {
                    cookies.add_private(Cookie::new("session_id", format!("{}", session.id)));
                    Ok("Signed in!".to_string())
                },
                Err(_e) => Err("Failed to create session".to_string())
            }
        }
    }
}

#[post("/login", data = "<login_user>")]
fn submit_login(db_conn: DbConn, login_user: Form<models::LoginUser>, cookies: Cookies) -> Redirect {
    let user_query = schema::users::table.filter(schema::users::username.eq(&login_user.username)).filter(schema::users::password_hash.crypt_eq(&login_user.password));
    match user_query.first::<models::User>(&*db_conn) {
        Ok(user) => {
            match update_session_logged_in_user(user, cookies, db_conn) {
                Ok(_) => Redirect::to(uri!(user: &login_user.username)),
                Err(_) => Redirect::to(uri!(login: "Failed to set session_id cookie.".to_string()))
            }
        },
        Err(_) => Redirect::to(uri!(login: "Couldn't authenticate user".to_string()))
    }
}

fn valid_username(username: &str) -> bool {
    Regex::new(r"^[0-9A-z]+$").unwrap().is_match(username)
}

#[post("/signup", data = "<new_user_form>")]
fn submit_signup(db_conn: DbConn, new_user_form: Form<models::NewUser>, cookies: Cookies) -> Redirect {
    let new_user = new_user_form.into_inner();
    let email = new_user.email.clone();
    let username = new_user.username.clone();
    match valid_username(&username) {
        true => {
            match validator::validate_email(email.clone()) {
                true => {
                    match new_user.password.len() >= 1 && new_user.password.len() <= 72 {
                        true => {
                            match new_user.execute(&*db_conn) {
                                Ok(_v) => {
                                    // TODO: Make it so custom models::NewUser insertion script is able to return models::User itself
                                    match schema::users::table.filter(schema::users::email.eq(email)).first::<models::User>(&*db_conn) {
                                        Ok(user) => {
                                            match update_session_logged_in_user(user, cookies, db_conn) {
                                                Ok(_) => Redirect::to(uri!(user: &username)),
                                                Err(_) => Redirect::to(uri!(signup: "Failed to set session_id cookie.".to_string()))
                                            }
                                        },
                                        Err(_) => Redirect::to(uri!(signup: "Failed to retrieve new user to add them to session".to_string()))
                                    }
                                },
                                Err(e) => {
                                    match &*e.to_string() {
                                        "duplicate key value violates unique constraint \"users_username_key\"" => Redirect::to(uri!(signup: "Duplicate username".to_string())),
                                        "duplicate key value violates unique constraint \"users_email_key\"" => Redirect::to(uri!(signup: "Duplicate email".to_string())),
                                        _ => {
                                            println!("Failed to add user to database: {}", e);
                                            Redirect::to(uri!(signup: "Failed to add user to database".to_string()))
                                        }
                                    }
                                }
                            }
                        },
                        false => Redirect::to(uri!(signup: "Invalid password".to_string()))
                    }
                }
                false => Redirect::to(uri!(signup: "Invalid email".to_string()))
            }
        },
        false => Redirect::to(uri!(signup: "Invalid username".to_string()))
    }
}

#[get("/user/<username>")]
fn user(username: String) -> String {
    "Not done".to_string()
}

fn main() {
    rocket::ignite()
        .mount("/", routes![
            home,
            favicon,
            sitemap,
            login,
            signup,
            submit_login,
            submit_signup,
            user,
        ])
        .attach(DbConn::fairing())
        .attach(Template::fairing())
        .launch();
}