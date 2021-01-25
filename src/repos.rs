use diesel::{
    prelude::*,
    PgConnection,
};

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
    apps,
    common::*,
    crypt_eq::CryptExpressionMethods,
    DbConn,
    schema,
    schema::repos,
    signed_in_context,
    users,
};

#[derive(Queryable, Serialize)]
pub struct Repo {
    pub id: i64,
    pub owner_id: i64,
    pub title: String,
    pub description: String,
    pub apps: Vec<i64>,
}

impl Repo {
    pub fn remove_app(&self, pg_conn: &PgConnection, app_id: i64) -> Result<(), String> {
        match repos::table.filter(repos::id.eq(self.id)).first::<Repo>(&*pg_conn) {
            Ok(mut repo) => {
                for (i, &test_app_id) in repo.apps.iter().enumerate() {
                    if test_app_id == app_id {
                        repo.apps.remove(i);
                        match diesel::update(repos::table.filter(repos::id.eq(repo.id))).set(repos::apps.eq(repo.apps)).execute(&*pg_conn) {
                            Ok(_) => return Ok(()),
                            Err(_) => return Err("Failed to remove app from repo".to_string())
                        }
                    }
                }
                return Err("Repo does not contain app".to_string())
            },
            Err(_) => Err("Failed to get updated repo to delete app from.".to_string())
        }
    }
}

#[derive(Serialize)]
pub struct CleanRepo {
    pub title: Cleaned,
    pub description: Cleaned,
}

impl CleanRepo {
    pub fn from_repo(repo: &Repo) -> CleanRepo {
        CleanRepo {
            title: Cleaned::new(&repo.title),
            description: Cleaned::new(&repo.description),
        }
    }

    pub fn from_vec(repos: &Vec<Repo>) -> Vec<CleanRepo> {
        let mut cleaned = Vec::new();
        
        for repo in repos {
            cleaned.push(CleanRepo::from_repo(&repo));
        }

        return cleaned;
    }
}

#[derive(FromForm)]
pub struct FormRepo {
    pub title: String,
    pub description: String,
}

impl FormRepo {
    pub fn to_new_repo(self, owner_id: i64) -> NewRepo {
        NewRepo {
            owner_id: owner_id,
            title: self.title,
            description: self.description,
        }
    }
}

#[derive(FromForm, Insertable)]
#[table_name = "repos"]
pub struct NewRepo {
    pub owner_id: i64,
    pub title: String,
    pub description: String,
}

pub fn get_by_title(pg_conn: &PgConnection, title: &str) -> Result<Repo, String> {
    match repos::table.filter(
        repos::title.eq(title)
    ).first::<Repo>(pg_conn) {
        Ok(repo) => Ok(repo),
        Err(e) => Err(format!("Failed to get repo by title {}", e))
    }
}

pub fn get_all(pg_conn: &PgConnection) -> Result<Vec<Repo>, String> {
    match repos::table.load::<Repo>(pg_conn) {
        Ok(repos) => Ok(repos),
        Err(e) => Err(format!("Failed to get repos {}", e))
    }
}

pub fn get_apps(pg_conn: &PgConnection, repo: &Repo) -> Result<Vec<apps::App>, String> {
    let mut apps = Vec::new();
    for app_id in &repo.apps {
        match schema::apps::table.filter(schema::apps::id.eq(app_id)).first(&*pg_conn) {
            Ok(app) => apps.push(app),
            Err(e) => {
                eprintln!("Failed to get app for repo. Repo ID: {}, App ID: {}, Error: {}", repo.id, app_id, e);
                return Err(format!("Failed to get app for repo. Repo ID: {}, App ID: {}", repo.id, app_id))
            }
        }
    }
    return Ok(apps);
}

#[get("/repos")]
pub fn repos(db_conn: DbConn, cookies: Cookies) -> Template {
    let (mut context, _, _) = signed_in_context(&*db_conn, cookies);
    match get_all(&*db_conn) {
        Ok(repos) => {
            context.insert("clean_repos", &CleanRepo::from_vec(&repos));
        },
        Err(_) => {}
    };
    Template::render("repos", &context)
}

#[get("/createRepo")]
pub fn create_repo(db_conn: DbConn, cookies: Cookies) -> Template {
    let (context, _, _) = signed_in_context(&*db_conn, cookies);
    Template::render("create_repo", &context)
}

#[post("/createRepo", data="<repo_form>")]
pub fn submit_repo(repo_form: Form<FormRepo>, db_conn: DbConn, cookies: Cookies) -> Result<Redirect, status::Custom<&str>> {
    let (_, _, user) = signed_in_context(&*db_conn, cookies);
    let form_repo = repo_form.into_inner();
    let new_repo;

    match user {
        Some(user) => {
            new_repo = form_repo.to_new_repo(user.id);
        },
        None => {
            return Err(status::Custom(Status::BadRequest, "Must be signed in"))
        }
    }

    if !validate_title(&new_repo.title) || new_repo.title.len() > 24 {return Err(status::Custom(Status::BadRequest, "Title must be 3-24 characters"))}
    if new_repo.description.len() > 256 {return Err(status::Custom(Status::BadRequest, "Description is too long - max 256 characters"))}

    match diesel::insert_into(repos::table).values(&new_repo).get_result::<Repo>(&*db_conn) {
        Ok(repo) => {
            Ok(Redirect::to(uri!(repo: repo.title)))
        },
        Err(e) => {
            match &*(e.to_string()) {
                "Failed to add repo to database duplicate key value violates unique constraint \"repos_title_unique_idx\"" => Err(status::Custom(Status::BadRequest, "Duplicate repo name")),
                _ => {
                    eprintln!("Failed to add repo to database {}", e);
                    Err(status::Custom(Status::InternalServerError, "Failed to add repo to database"))
                }
            }
        }
    }
}

#[get("/repos/<title>")]
pub fn repo(title: String, db_conn: DbConn, cookies: Cookies) -> Result<Template, status::NotFound<String>> {
    let (mut context, _, _) = signed_in_context(&*db_conn, cookies);

    match get_by_title(&*db_conn, &title) {
        Ok(repo) => {
            context.insert("repo", &repo);
            context.insert("clean_repo", &CleanRepo::from_repo(&repo));
            match users::get(&*db_conn, repo.id) {
                Ok(owner) => {
                    context.insert("owner", &owner);
                    context.insert("clean_owner", &users::CleanUser::from_user(&owner));
                },
                _ => {}
            }
            match get_apps(&*db_conn, &repo) {
                Ok(apps) => {
                    context.insert("apps", &apps);
                    context.insert("clean_apps", &apps::CleanApp::from_vec(&apps));
                },
                _ => {}
            }
            Ok(Template::render("repo", &context))
        },
        Err(_) => Err(status::NotFound("Repo not found".to_owned()))
    }
}

#[post("/repos/<title>/delete", data = "<login_user>")]
pub fn delete_repo(title: String, db_conn: DbConn, login_user: Form<users::LoginUser>) -> Result<Redirect, status::Custom<&'static str>> {
    match schema::users::table.filter(schema::users::username.eq(&login_user.username)).filter(schema::users::password_hash.crypt_eq(&login_user.password)).first::<users::User>(&*db_conn) {
        Ok(user) => {
            match get_by_title(&*db_conn, &title) {
                Ok(repo) => {
                    if repo.owner_id == user.id {
                        match diesel::delete(repos::table.filter(repos::id.eq(repo.id))).execute(&*db_conn) {
                            Ok(_) => Ok(Redirect::to(uri!(super::home))),
                            Err(_) => Err(status::Custom(Status::InternalServerError, "Failed to delete repo"))
                        }
                    } else {
                        return Err(status::Custom(Status::Forbidden, "You don't have permission to delete this repo"))
                    }
                },
                Err(_) => Err(status::Custom(Status::NotFound, "Repo not found"))
            }
        },
        Err(_) => Err(status::Custom(Status::Forbidden, "Failed to authenticate user"))
    }
}

#[derive(FromForm)]
pub struct AddAppForum {
    title: String
}

#[post("/repos/<title>/addApp", data = "<add_app_forum>")]
pub fn add_app(title: String, db_conn: DbConn, cookies: Cookies, add_app_forum: Form<AddAppForum>) -> Result<Redirect, status::Custom<&'static str>> {
    match get_by_title(&*db_conn, &title) {
        Ok(mut repo) => {
            match users::get_from_cookies(&*db_conn, cookies) {
                Ok (user) => {
                    if user.id == repo.owner_id {
                        match apps::get_by_title(&*db_conn, &add_app_forum.title) {
                            Ok(app) => {
                                repo.apps.push(app.id);
                                match diesel::update(repos::table.filter(repos::id.eq(repo.id))).set(repos::apps.eq(repo.apps)).execute(&*db_conn) {
                                    Ok(_) => Ok(Redirect::to(uri!(repo: title))),
                                    Err(_) => Err(status::Custom(Status::InternalServerError, "Failed to add app."))
                                }
                            },
                            Err(_) => Err(status::Custom(Status::NotFound, "Failed to get app id from title"))
                        }
                    } else {
                        return Err(status::Custom(Status::Forbidden, "You don't have permission to add an app to this repo"))
                    }
                },
                Err(_) => Err(status::Custom(Status::Forbidden, "Failed to authenticate user"))
            }
        },
        Err(_) => Err(status::Custom(Status::Forbidden, "Failed to authenticate user"))
    }
}