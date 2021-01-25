use diesel::{
    prelude::*,
    PgConnection,
};

use regex::Regex;
use rocket::{
    http::{
        Cookies,
        Status,
    },
    request::Form,
    response::{
        Redirect,
        status,
    },
    uri,
};

use rocket_contrib::templates::Template;
use serde::Serialize;

use super::{
    common::*,
    crypt_eq::CryptExpressionMethods,
    DbConn,
    repos,
    schema,
    schema::apps,
    signed_in_context,
    users,
};

#[derive(Queryable, Serialize)]
pub struct App {
    pub id: i64,
    pub owner_id: i64,
    pub title: String,
    pub description: String,
    pub domain: String,
    pub token: String,
    pub connected: bool,
    pub connected_error: String,
}

#[derive(Serialize)]
pub struct CleanApp {
    pub title: Cleaned,
    pub description: Cleaned,
    pub domain: Cleaned,
    pub token: Cleaned,
    pub connected_error: Cleaned,
}

impl CleanApp {
    pub fn from_app(app: &App) -> CleanApp {
        CleanApp {
            title: Cleaned::new(&app.title),
            description: Cleaned::new(&app.description),
            domain: Cleaned::new(&app.domain),
            token: Cleaned::new(&app.token),
            connected_error: Cleaned::new(&app.connected_error),
        }
    }

    pub fn from_vec(apps: &Vec<App>) -> Vec<CleanApp> {
        let mut cleaned = Vec::new();
        
        for app in apps {
            cleaned.push(CleanApp::from_app(&app));
        }

        return cleaned;
    }
}

#[derive(FromForm)]
pub struct FormApp {
    pub title: String,
    pub description: String,
    pub domain: String,
    pub token: String
}

impl FormApp {
    pub fn to_new_app(self, owner_id: i64) -> NewApp {
        NewApp {
            owner_id: owner_id,
            title: self.title,
            description: self.description,
            domain: self.domain,
            token: self.token
        }
    }
}

#[derive(FromForm, Insertable)]
#[table_name = "apps"]
pub struct NewApp {
    pub owner_id: i64,
    pub title: String,
    pub description: String,
    pub domain: String,
    pub token: String
}

pub fn get_by_title(pg_conn: &PgConnection, title: &str) -> Result<App, String> {
    match apps::table.filter(
        apps::title.eq(title)
    ).first::<App>(pg_conn) {
        Ok(app) => Ok(app),
        Err(e) => Err(format!("Failed to get app by title {}", e))
    }
}

pub fn get_all(pg_conn: &PgConnection) -> Result<Vec<App>, String> {
    match apps::table.load::<App>(pg_conn) {
        Ok(apps) => Ok(apps),
        Err(e) => Err(format!("Failed to get apps {}", e))
    }
}

#[get("/apps")]
pub fn apps(db_conn: DbConn, cookies: Cookies) -> Template {
    let (mut context, _, _) = signed_in_context(&*db_conn, cookies);
    match get_all(&*db_conn) {
        Ok(apps) => {
            context.insert("clean_apps", &CleanApp::from_vec(&apps));
        },
        Err(_) => {}
    };
    Template::render("apps", &context)
}

fn validate_domain(domain: &str) -> bool {
    let valid_formatting = Regex::new(
        r"^https://([a-zA-Z0-9][a-zA-Z0-9-]{1,61}[a-zA-Z0-9].)?[a-zA-Z0-9][a-zA-Z0-9-]{1,61}[a-zA-Z0-9].[a-zA-Z]{2,3}(:[0-9]{1,5})?$"
    ).unwrap().is_match(domain);
    
    let valid_port = match Regex::new(
        r"[0-9]{1,5}$"
    ).unwrap().find(domain) {
        Some(port_match) => {
            return port_match.as_str().parse::<i32>().unwrap() < 65536
        }
        None => true
    };
    
    return valid_formatting && valid_port
}

#[get("/createApp")]
pub fn create_app(db_conn: DbConn, cookies: Cookies) -> Template {
    let (context, _, _) = signed_in_context(&*db_conn, cookies);
    Template::render("create_app", &context)
}

#[post("/createApp", data="<app_form>")]
pub fn submit_app(app_form: Form<FormApp>, db_conn: DbConn, cookies: Cookies) -> Result<Redirect, status::Custom<&str>> {
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

    match diesel::insert_into(apps::table).values(&new_app).get_result::<App>(&*db_conn) {
        Ok(app) => {
            Ok(Redirect::to(uri!(app: app.title)))
        },
        Err(e) => {
            match &*(e.to_string()) {
                "Failed to add app to database duplicate key value violates unique constraint \"apps_title_unique_idx\"" => Err(status::Custom(Status::BadRequest, "Duplicate app name")),
                _ => {
                    eprintln!("Failed to add app to database {}", e);
                    Err(status::Custom(Status::InternalServerError, "Failed to add app to database"))
                }
            }
        }
    }
}

#[get("/apps/<title>")]
pub fn app(title: String, db_conn: DbConn, cookies: Cookies) -> Result<Template, status::NotFound<String>> {
    let (mut context, _, _) = signed_in_context(&*db_conn, cookies);

    match get_by_title(&*db_conn, &title) {
        Ok(app) => {
            context.insert("app", &app);
            context.insert("clean_app", &CleanApp::from_app(&app));
            match users::get(&*db_conn, app.id) {
                Ok(owner) => {
                    context.insert("owner", &owner);
                    context.insert("clean_owner", &users::CleanUser::from_user(&owner));
                },
                _ => {}
            }
            Ok(Template::render("app", &context))
        },
        Err(_) => Err(status::NotFound("App not found".to_owned()))
    }
}

#[post("/apps/<title>/delete", data = "<login_user>")]
pub fn delete_app(title: String, db_conn: DbConn, login_user: Form<users::LoginUser>) -> Result<Redirect, status::Custom<String>> {
    match schema::users::table.filter(schema::users::username.eq(&login_user.username)).filter(schema::users::password_hash.crypt_eq(&login_user.password)).first::<users::User>(&*db_conn) {
        Ok(user) => {
            match get_by_title(&*db_conn, &title) {
                Ok(app) => {
                    if app.owner_id == user.id {
                        match schema::repos::table.filter(schema::repos::apps.contains(vec!(app.id))).load::<repos::Repo>(&*db_conn) {
                            Ok(repos) => {
                                for repo in repos {
                                    match repo.remove_app(&*db_conn, app.id) {
                                        Ok(_) => {},
                                        Err(_) => return Err(status::Custom(Status::InternalServerError, format!("Failed to remove app from repo: {}.", repo.id)))
                                    }
                                }
                                match diesel::delete(apps::table.filter(apps::id.eq(app.id))).execute(&*db_conn) {
                                    Ok(_) => Ok(Redirect::to(uri!(super::home))),
                                    Err(_) => Err(status::Custom(Status::InternalServerError, "Failed to delete app".to_string()))
                                }
                            },
                            Err(_) => Err(status::Custom(Status::InternalServerError, "Couldn't retrieve repos that would be affected by deletion.".to_string()))
                        }
                    } else {
                        return Err(status::Custom(Status::Forbidden, "You don't have permission to delete this app".to_string()))
                    }
                },
                Err(_) => Err(status::Custom(Status::NotFound, "App not found".to_string()))
            }
        },
        Err(_) => Err(status::Custom(Status::Forbidden, "Failed to authenticate user".to_string()))
    }
}