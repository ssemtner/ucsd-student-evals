mod parser;
pub mod sids;

pub use parser::*;
use sqlx::{query, Pool, Postgres};

use anyhow::Result;

async fn get_or_create_term_id(conn: &Pool<Postgres>, name: String) -> Result<i32> {
    let id = query!(
        "
            INSERT INTO terms (name)
            VALUES ($1)
            ON CONFLICT (name) DO UPDATE
            SET name = terms.name
            RETURNING id
        ",
        name
    )
    .fetch_one(conn)
    .await?
    .id;

    Ok(id)
}

async fn get_or_create_instructor_id(conn: &Pool<Postgres>, name: String) -> Result<i32> {
    let id = query!(
        "
            INSERT INTO instructors (name)
            VALUES ($1)
            ON CONFLICT (name) DO UPDATE
            SET name = instructors.name
            RETURNING id
        ",
        name
    )
    .fetch_one(conn)
    .await?
    .id;

    Ok(id)
}
