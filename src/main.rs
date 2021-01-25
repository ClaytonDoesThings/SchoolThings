#![feature(proc_macro_hygiene, decl_macro)]

use std::path::Path;

#[macro_use] extern crate rocket;
use rocket::{
    http::Cookies,
    response::{
        NamedFile,
        status,
    },
};

#[macro_use] extern crate rocket_contrib;
use rocket_contrib::templates::{
    Template,
};

#[macro_use] extern crate diesel;
use diesel::PgConnection;

pub mod apps;
pub mod common;
pub mod crypt_eq;
pub mod repos;
pub mod schema;
pub mod sessions;
pub mod users;

use crate::common::*;

#[database("postgres")]
pub struct DbConn(PgConnection);

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
fn sitemap(db_conn: DbConn) -> Template {
    let mut context = default_context();

    match apps::get_all(&*db_conn) {
        Ok(apps) => context.insert("clean_apps", &apps::CleanApp::from_vec(&apps)),
        _ => {}
    };
    match users::get_all(&*db_conn) {
        Ok(users) => context.insert("clean_users", &users::CleanUser::from_vec(&users)),
        _ => {}
    };

    Template::render("sitemap", &context)
}

fn main() {
    rocket::ignite()
        .mount("/", routes![
            home,
            favicon,
            sitemap,
            users::login,
            users::signup,
            users::submit_login,
            users::submit_signup,
            users::signout,
            users::user_profile,
            apps::apps,
            apps::create_app,
            apps::submit_app,
            apps::app,
            apps::delete_app,
            repos::repos,
            repos::create_repo,
            repos::submit_repo,
            repos::repo,
            repos::delete_repo,
            repos::add_app,
        ])
        .attach(DbConn::fairing())
        .attach(Template::fairing())
        .launch();
}