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
    pub fn new(user: User) -> CleanUser {
        CleanUser {
            username: Cleaned::new(&user.username),
            email: Cleaned::new(&user.email),
        }
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