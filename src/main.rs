#![feature(proc_macro_hygiene, decl_macro)]

use ammonia::clean_text;
use regex::Regex;
use std::path::Path;
use validator::{
    validate_email,
};

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

fn signed_in_context(pg_conn: &PgConnection, cookies: Cookies) -> (Context, Option<models::Session>, Option<models::User>) {
    let mut context = default_context();
    match db::get_session_from_cookies(&pg_conn, cookies) {
        Ok(session) => {
            match db::get_user_from_session(&pg_conn, &session) {
                Ok(user) => {
                    context.insert("user", &user);
                    context.insert("clean_user", &models::CleanUser::new(&user));
                    (context, Some(session), Some(user))
                },
                Err(_) => (context, Some(session), None)
            }
        },
        Err(_) => (context, None, None)
    }
}

#[get("/")]
fn home(db_conn: DbConn, cookies: Cookies) -> Template {
    let (context, _, _) = signed_in_context(&*db_conn, cookies);
    Template::render("home", &context)
}

#[get("/favicon.ico")]
fn favicon() -> Result<NamedFile, status::NotFound<String>> {
    let path = Path::new("favicon.ico");
    NamedFile::open(&path).map_err(|e| status::NotFound(e.to_string()))
}

#[get("/sitemap.xml")]
fn sitemap() -> Template { Template::render("sitemap", &default_context()) }

#[get("/login?<error>&<username>")]
fn login(error: Option<String>, username: Option<String>, db_conn: DbConn, cookies: Cookies) -> Template {
    let (mut context, _, _) = signed_in_context(&*db_conn, cookies);
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
    let (mut context, _, _) = signed_in_context(&*db_conn, cookies);
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
                        Ok(_) => Ok("Signed in!".to_string()),
                        Err(_) => Err("Failed to update session".to_string())
                    }
                },
                Err(_) => Err("Failed to parse session_id cookie".to_string())
            }
        },
        None => {
            match diesel::insert_into(schema::sessions::table).values(&models::NewSession::new(user.id)).get_result::<models::Session>(&*db_conn) {
                Ok(session) => {
                    cookies.add_private(Cookie::new("session_id", format!("{}", session.id)));
                    Ok("Signed in!".to_string())
                },
                Err(_) => Err("Failed to create session".to_string())
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

fn validate_title(title: &str) -> bool {
    Regex::new(r"^[0-9A-Za-z][0-9A-Za-z_-]{1,}[0-9A-Za-z]$").unwrap().is_match(title)
}

#[post("/signup", data = "<new_user_form>")]
fn submit_signup(db_conn: DbConn, new_user_form: Form<models::NewUser>, cookies: Cookies) -> Redirect {
    let new_user = new_user_form.into_inner();
    let email = new_user.email.clone();
    let username = new_user.username.clone();
    match validate_title(&username) && username.len() <= 12 {
        true => {
            match validate_email(&email) {
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
                                            eprintln!("Failed to add user to database: {}", e);
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
    let (mut context, _, _) = signed_in_context(&*db_conn, cookies);
    match db::get_user_by_username(&*db_conn, username) {
        Ok(user) => {
            context.insert("profile", &user);
            context.insert("profile_cleaned", &models::CleanUser::new(&user));
            Ok(Template::render("user_profile", &context))
        },
        Err(_) => Err(status::NotFound("Couldn't find user".to_string()))
    }
}

#[get("/apps")]
fn apps(db_conn: DbConn, cookies: Cookies) -> Template {
    let (mut context, _, _) = signed_in_context(&*db_conn, cookies);
    match db::get_apps(&*db_conn) {
        Ok(apps) => {
            context.insert("clean_apps", &models::CleanApp::from_vec(&apps));
        },
        Err(_) => {}
    };
    Template::render("apps", &context)
}

fn validate_domain(domain: &str) -> bool {
    Regex::new(
        r"^https://([a-zA-Z0-9][a-zA-Z0-9-]{1,61}[a-zA-Z0-9].)?[a-zA-Z0-9][a-zA-Z0-9-]{1,61}[a-zA-Z0-9].[a-zA-Z]{2,3}(:[0-9]{1,5})?$"
    ).unwrap().is_match(domain) && (
        Regex::new(
            r"[0-9]{1,5}$"
        ).unwrap()
        .find(domain).unwrap().as_str()
        .parse::<i32>().unwrap() < 65536
    )
}

#[get("/createApp")]
fn create_app(db_conn: DbConn, cookies: Cookies) -> Template {
    let (context, _, _) = signed_in_context(&*db_conn, cookies);
    Template::render("create_app", &context)
}

#[post("/createApp", data="<app_form>")]
fn submit_app(app_form: Form<models::FormApp>, db_conn: DbConn, cookies: Cookies) -> Result<Redirect, status::Custom<&str>> {
    let (_, _, user) = signed_in_context(&*db_conn, cookies);
    let form_app = app_form.into_inner();
    let new_app;

    match user {
        Some(user) => {
            new_app = form_app.to_new_app(user.id);
        },
        None => {
            return Err(status::Custom(Status::BadRequest, "Must be signed in"))
        }
    }

    if !validate_domain(&new_app.domain) {return Err(status::Custom(Status::BadRequest, "Invalid domain"))}
    if !validate_title(&new_app.title) || new_app.title.len() > 24 {return Err(status::Custom(Status::BadRequest, "Title must be 3-24 characters"))}
    if new_app.description.len() > 256 {return Err(status::Custom(Status::BadRequest, "Description is too long - max 256 characters"))}
    if new_app.token.len() != 60 {return Err(status::Custom(Status::BadRequest, "Token must be exactly 60 characters"))}

    match diesel::insert_into(schema::apps::table).values(&new_app).get_result::<models::App>(&*db_conn) {
        Ok(app) => {
            Ok(Redirect::to(uri!(app: app.title)))
        },
        Err(e) => {
            match &*(e.to_string()) {
                "Failed to add app to database duplicate key value violates unique constraint \"apps_title_key\"" => Err(status::Custom(Status::BadRequest, "Duplicate app name")),
                _ => {
                    eprintln!("Failed to add app to database {}", e);
                    Err(status::Custom(Status::InternalServerError, "Failed to add app to database"))
                }
            }
        }
    }
}

#[get("/apps/<title>")]
fn app(title: String, db_conn: DbConn, cookies: Cookies) -> Result<Template, status::NotFound<String>> {
    let (mut context, _, _) = signed_in_context(&*db_conn, cookies);

    match db::get_app_by_title(&*db_conn, &title) {
        Ok(app) => {
            context.insert("app", &app);
            context.insert("clean_app", &models::CleanApp::from_app(&app));
            Ok(Template::render("app", &context))
        },
        Err(_) => Err(status::NotFound("App not found".to_owned()))
    }
}

#[post("/apps/<title>/delete", data = "<login_user>")]
fn delete_app(title: String, db_conn: DbConn, login_user: Form<models::LoginUser>) -> Result<Redirect, status::Custom<&'static str>> {
    match schema::users::table.filter(schema::users::username.eq(&login_user.username)).filter(schema::users::password_hash.crypt_eq(&login_user.password)).first::<models::User>(&*db_conn) {
        Ok(user) => {
            match db::get_app_by_title(&*db_conn, &title) {
                Ok(app) => {
                    if app.owner_id == user.id {
                        match diesel::delete(schema::apps::table.filter(schema::apps::id.eq(app.id))).execute(&*db_conn) {
                            Ok(_) => Ok(Redirect::to(uri!(home))),
                            Err(_) => Err(status::Custom(Status::InternalServerError, "Failed to delete app"))
                        }
                    } else {
                        return Err(status::Custom(Status::Forbidden, "You don't have permission to delete this app"))
                    }
                },
                Err(_) => Err(status::Custom(Status::NotFound, "App not found"))
            }
        },
        Err(_) => Err(status::Custom(Status::Forbidden, "Failed to authenticate user"))
    }
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
            submit_app,
            app,
            delete_app,
        ])
        .attach(DbConn::fairing())
        .attach(Template::fairing())
        .launch();
}