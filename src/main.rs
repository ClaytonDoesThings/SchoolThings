#![feature(proc_macro_hygiene, decl_macro)]

use ammonia::clean_text;
use regex::Regex;
use std::path::Path;

#[macro_use] extern crate rocket;
use rocket::{
    http::{
        Cookie,
        Cookies,
        Status,
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
use diesel::{
    prelude::*,
    PgConnection,
};
pub mod schema;
pub mod models;
pub mod crypt_eq;
pub mod db;
use crate::crypt_eq::CryptExpressionMethods;

#[database("postgres")]
struct DbConn(PgConnection);

fn default_context() -> Context {
    let mut context = Context::new();
    context.insert("domain", "https://schoolthings.xyz");
    return context;
}

fn signed_in_context(pg_conn: &PgConnection, cookies: Cookies) -> Context {
    let mut context = default_context();
    match db::get_session_from_cookies(&pg_conn, cookies) {
        Ok(session) => {
            match db::get_user_from_session(&pg_conn, session) {
                Ok(user) => {
                    context.insert("user", &user);
                    context.insert("clean_user", &models::CleanUser::new(user));
                },
                Err(_) => {}
            }
        },
        Err(_) => {}
    }
    return context;
}

#[get("/")]
fn home(db_conn: DbConn, cookies: Cookies) -> Template { Template::render("home", &signed_in_context(&*db_conn, cookies)) }

#[get("/favicon.ico")]
fn favicon() -> Result<NamedFile, status::NotFound<String>> {
    let path = Path::new("favicon.ico");
    NamedFile::open(&path).map_err(|e| status::NotFound(e.to_string()))
}

#[get("/sitemap.xml")]
fn sitemap() -> Template { Template::render("sitemap", &default_context()) }

#[get("/login?<error>&<username>")]
fn login(error: Option<String>, username: Option<String>, db_conn: DbConn, cookies: Cookies) -> Template {
    let mut context = signed_in_context(&*db_conn, cookies);
    match error {
        Some(error) => context.insert("error", &clean_text(&error)),
        None => {}
    }
    match username {
        Some(username) => context.insert("username", &clean_text(&username)),
        None => {}
    }
    Template::render("login", &context)
}

#[get("/signup?<error>&<username>&<email>")]
fn signup(error: Option<String>, username: Option<String>, email: Option<String>, db_conn: DbConn, cookies: Cookies) -> Template {
    let mut context = signed_in_context(&*db_conn, cookies);
    match error {
        Some(error) => context.insert("error", &clean_text(&error)),
        None => {}
    }
    match username {
        Some(username) => context.insert("username", &clean_text(&username)),
        None => {}
    }
    match email {
        Some(email) => context.insert("email", &clean_text(&email)),
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
    let username = &login_user.username;
    let user_query = schema::users::table.filter(schema::users::username.eq(&username)).filter(schema::users::password_hash.crypt_eq(&login_user.password));
    match user_query.first::<models::User>(&*db_conn) {
        Ok(user) => {
            match update_session_logged_in_user(user, cookies, db_conn) {
                Ok(_) => Redirect::to(uri!(user_profile: username)),
                Err(_) => Redirect::to(uri!(login: "Failed to set session_id cookie.".to_string(), username))
            }
        },
        Err(_) => Redirect::to(uri!(login: "Couldn't authenticate user".to_string(), username))
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
            match validator::validate_email(&email) {
                true => {
                    match new_user.password.len() >= 1 && new_user.password.len() <= 72 {
                        true => {
                            match new_user.execute(&*db_conn) {
                                Ok(_v) => {
                                    // TODO: Make it so custom models::NewUser insertion script is able to return models::User itself
                                    match schema::users::table.filter(schema::users::email.eq(&email)).first::<models::User>(&*db_conn) {
                                        Ok(user) => {
                                            match update_session_logged_in_user(user, cookies, db_conn) {
                                                Ok(_) => Redirect::to(uri!(user_profile: &username)),
                                                Err(_) => Redirect::to(uri!(signup: "Failed to set session_id cookie.".to_string(), username, email))
                                            }
                                        },
                                        Err(_) => Redirect::to(uri!(signup: "Failed to retrieve new user to add them to session".to_string(), username, email))
                                    }
                                },
                                Err(e) => {
                                    match &*e.to_string() {
                                        "duplicate key value violates unique constraint \"users_username_key\"" => Redirect::to(uri!(signup: "Duplicate username".to_string(), username, email)),
                                        "duplicate key value violates unique constraint \"users_email_key\"" => Redirect::to(uri!(signup: "Duplicate email".to_string(), username, email)),
                                        _ => {
                                            println!("Failed to add user to database: {}", e);
                                            Redirect::to(uri!(signup: "Failed to add user to database".to_string(), username, email))
                                        }
                                    }
                                }
                            }
                        },
                        false => Redirect::to(uri!(signup: "Invalid password".to_string(), username, email))
                    }
                }
                false => Redirect::to(uri!(signup: "Invalid email".to_string(), username, email))
            }
        },
        false => Redirect::to(uri!(signup: "Invalid username".to_string(), username, email))
    }
}

#[post("/signout")]
fn signout(db_conn: DbConn, cookies: Cookies) -> Result<Redirect, status::Custom<String>> {
    match db::get_session_from_cookies(&*db_conn, cookies) {
        Ok(session) => {
            match session.logged_in_user {
                Some(_) => {
                    match diesel::update(schema::sessions::table.filter(schema::sessions::id.eq(session.id))).set(schema::sessions::logged_in_user.eq::<Option<i64>>(None)).execute(&*db_conn) {
                        Ok(_) => Ok(Redirect::to(uri!(home))),
                        Err(_) => {
                            Err(status::Custom(Status::InternalServerError, "Failed to update session".to_string()))
                        }
                    }
                },
                None => Err(status::Custom(Status::BadRequest, "Session not signed in".to_string()))
            }
        },
        Err(_) => {
            Err(status::Custom(Status::InternalServerError, "Failed to get session from cookies".to_string()))
        }
    }
}

#[get("/users/<username>")]
fn user_profile(username: String, db_conn: DbConn, cookies: Cookies) -> Result<Template, status::NotFound<String>> {
    let mut context = signed_in_context(&*db_conn, cookies);
    match db::get_user_by_username(&*db_conn, username) {
        Ok(user) => {
            context.insert("profile", &user);
            context.insert("profile_cleaned", &models::CleanUser::new(user));
            Ok(Template::render("user_profile", &context))
        },
        Err(e) => Err(status::NotFound(e))
    }
}

#[get("/apps")]
fn apps(db_conn: DbConn, cookies: Cookies) -> Template {
    let context = signed_in_context(&*db_conn, cookies);
    Template::render("apps", &context)
}

#[get("/apps/create")]
fn create_app(db_conn: DbConn, cookies: Cookies) -> Template {
    let context = signed_in_context(&*db_conn, cookies);
    Template::render("create_app", &context)
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
            signout,
            user_profile,
            apps,
            create_app,
        ])
        .attach(DbConn::fairing())
        .attach(Template::fairing())
        .launch();
}