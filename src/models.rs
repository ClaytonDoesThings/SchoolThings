use ammonia::clean_text;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::{QueryFragment, AstPass};
use diesel::sql_types::Text;
use percent_encoding::{
    NON_ALPHANUMERIC,
    percent_encode,
};
use serde::Serialize;
use super::schema::*;

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