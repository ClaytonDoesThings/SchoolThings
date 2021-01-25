use ammonia::clean_text;

use rocket::{
    http::{
        Cookie,
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

use diesel::{
    prelude::*,
    pg::Pg,
    PgConnection,
    query_builder::{
        QueryFragment,
        AstPass,
    },
    sql_types::Text,
};

use serde::Serialize;

use super::{
    common::*,
    crypt_eq::CryptExpressionMethods,
    DbConn,
    schema,
    sessions,
    signed_in_context,
};

use validator::validate_email;


#[derive(Queryable, Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub password_hash: String,
}

#[derive(Serialize)]
pub struct CleanUser {
    pub username: Cleaned,
    pub email: Cleaned,
}

impl CleanUser {
    pub fn from_user(user: &User) -> CleanUser {
        CleanUser {
            username: Cleaned::new(&user.username),
            email: Cleaned::new(&user.email),
        }
    }

    pub fn from_vec(users: &Vec<User>) -> Vec<CleanUser> {
        let mut cleaned = Vec::new();
        
        for user in users {
            cleaned.push(CleanUser::from_user(&user));
        }

        return cleaned;
    }
}

#[derive(FromForm)]
pub struct LoginUser {
    pub username: String,
    pub password: String,
}

#[derive(FromForm, QueryId)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub password: String,
}

impl RunQueryDsl<PgConnection> for NewUser {}

impl QueryFragment<Pg> for NewUser {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql("INSERT INTO users (username, email, password_hash) VALUES (");
        out.push_bind_param::<Text, _>(&self.username)?;
        out.push_sql(", ");
        out.push_bind_param::<Text, _>(&self.email)?;
        out.push_sql(", crypt(");
        out.push_bind_param::<Text, _>(&self.password)?;
        out.push_sql(", gen_salt('bf')))");
        Ok(())
    }
}

pub fn get(pg_conn: &PgConnection, user_id: i64) -> Result<User, String> {
    match schema::users::table.find(user_id).first::<User>(pg_conn) {
        Ok(user) => Ok(user),
        Err(e) => Err(format!("Failed to get user: {}", e))
    }
}

pub fn get_by_username(pg_conn: &PgConnection, username: String) -> Result<User, String> {
    match schema::users::table.filter(schema::users::username.eq(username)).first::<User>(pg_conn) {
        Ok(user) => Ok(user),
        Err(e) => Err(format!("Failed to get user by username {}", e))
    }
}

pub fn get_from_session(pg_conn: &PgConnection, session: &sessions::Session) -> Result<User, String> {
    match session.logged_in_user {
        Some(logged_in_user) => get(pg_conn, logged_in_user),
        None => Err("Session is not logged in".to_string())
    }
}

pub fn get_from_cookies(pg_conn: &PgConnection, cookies: Cookies) -> Result<User, String> {
    match sessions::get_from_cookies(pg_conn, cookies) {
        Ok(session) => {
            match session.logged_in_user {
                Some(logged_in_user) => get(pg_conn, logged_in_user),
                None => Err("Session is not logged in".to_string())
            }
        },
        Err(_) => Err("Failed to get session from cookies".to_string())
    }
}

pub fn get_all(pg_conn: &PgConnection) -> Result<Vec<User>, String> {
    match schema::users::table.load::<User>(pg_conn) {
        Ok(users) => Ok(users),
        Err(e) => Err(format!("Failed to get users {}", e))
    }
}

#[get("/login?<error>&<username>")]
pub fn login(error: Option<String>, username: Option<String>, db_conn: DbConn, cookies: Cookies) -> Template {
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
pub fn signup(error: Option<String>, username: Option<String>, email: Option<String>, db_conn: DbConn, cookies: Cookies) -> Template {
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

fn update_session_logged_in_user(user: User, mut cookies: Cookies, db_conn: DbConn) -> Result<String, String> {
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
            match diesel::insert_into(schema::sessions::table).values(&sessions::NewSession::new(user.id)).get_result::<sessions::Session>(&*db_conn) {
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
pub fn submit_login(db_conn: DbConn, login_user: Form<LoginUser>, cookies: Cookies) -> Redirect {
    let username = &login_user.username;
    let user_query = schema::users::table.filter(schema::users::username.eq(&username)).filter(schema::users::password_hash.crypt_eq(&login_user.password));
    match user_query.first::<User>(&*db_conn) {
        Ok(user) => {
            match update_session_logged_in_user(user, cookies, db_conn) {
                Ok(_) => Redirect::to(uri!(user_profile: username)),
                Err(_) => Redirect::to(uri!(login: "Failed to set session_id cookie.".to_string(), username))
            }
        },
        Err(_) => Redirect::to(uri!(login: "Couldn't authenticate user".to_string(), username))
    }
}

#[post("/signup", data = "<new_user_form>")]
pub fn submit_signup(db_conn: DbConn, new_user_form: Form<NewUser>, cookies: Cookies) -> Redirect {
    let new_user = new_user_form.into_inner();
    let email = new_user.email.clone();
    let username = new_user.username.clone();
    match validate_title(&username) && username.len() <= 24 {
        true => {
            match validate_email(&email) {
                true => {
                    match new_user.password.len() >= 1 && new_user.password.len() <= 72 {
                        true => {
                            match new_user.execute(&*db_conn) {
                                Ok(_v) => {
                                    // TODO: Make it so custom models::NewUser insertion script is able to return models::User itself
                                    match schema::users::table.filter(schema::users::email.eq(&email)).first::<User>(&*db_conn) {
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
                                        "duplicate key value violates unique constraint \"users_username_unique_idx\"" => Redirect::to(uri!(signup: "Duplicate username".to_string(), username, email)),
                                        "duplicate key value violates unique constraint \"users_email_unique_idx\"" => Redirect::to(uri!(signup: "Duplicate email".to_string(), username, email)),
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
pub fn signout(db_conn: DbConn, cookies: Cookies) -> Result<Redirect, status::Custom<String>> {
    match sessions::get_from_cookies(&*db_conn, cookies) {
        Ok(session) => {
            match session.logged_in_user {
                Some(_) => {
                    match diesel::update(schema::sessions::table.filter(schema::sessions::id.eq(session.id))).set(schema::sessions::logged_in_user.eq::<Option<i64>>(None)).execute(&*db_conn) {
                        Ok(_) => Ok(Redirect::to(uri!(super::home))),
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
pub fn user_profile(username: String, db_conn: DbConn, cookies: Cookies) -> Result<Template, status::NotFound<String>> {
    let (mut context, _, _) = signed_in_context(&*db_conn, cookies);
    match get_by_username(&*db_conn, username) {
        Ok(user) => {
            context.insert("profile", &user);
            context.insert("profile_cleaned", &CleanUser::from_user(&user));
            Ok(Template::render("user_profile", &context))
        },
        Err(_) => Err(status::NotFound("Couldn't find user".to_string()))
    }
}