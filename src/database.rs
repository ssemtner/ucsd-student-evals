use crate::settings;
use anyhow::Result;
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::{FromRow, Pool, Postgres};

pub async fn establish_connection() -> Result<Pool<Postgres>, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&settings().database_url)
        .await
}

#[derive(FromRow, Serialize, Debug, PartialEq)]
pub struct Unit {
    pub id: i32,
    pub name: String,
}

#[derive(FromRow, Serialize, Debug, PartialEq)]
pub struct Course {
    pub code: String,
    pub name: String,
    pub unit_id: i32,
}

#[derive(FromRow, Serialize, Debug, PartialEq)]
pub struct SectionId {
    pub sid: i32,
    pub course_code: String,
}

#[derive(FromRow, Serialize, Debug, PartialEq)]
pub struct Term {
    pub id: i32,
    pub name: String,
}

#[derive(FromRow, Serialize, Debug, PartialEq)]
pub struct Instructor {
    pub id: i32,
    pub name: String,
}
