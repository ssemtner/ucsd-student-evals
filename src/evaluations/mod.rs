mod parser;
mod sids;

pub use parser::*;

use crate::database::{NewInstructor, NewTerm};
use crate::schema;
use anyhow::Result;
use diesel::dsl::insert_into;
use diesel::prelude::*;
use diesel::{QueryDsl, RunQueryDsl, SqliteConnection};

fn get_or_create_term_id(conn: &mut SqliteConnection, name: String) -> Result<i32> {
    let id = schema::terms::table
        .filter(schema::terms::name.eq(&name))
        .select(schema::terms::id)
        .get_result::<i32>(conn)
        .optional()?;

    match id {
        Some(id) => Ok(id),
        None => {
            let id = insert_into(schema::terms::table)
                .values(NewTerm { name })
                .returning(schema::terms::id)
                .execute(conn)?;
            Ok(id as i32)
        }
    }
}

fn get_or_create_instructor_id(conn: &mut SqliteConnection, name: String) -> Result<i32> {
    let id = schema::instructors::table
        .filter(schema::instructors::name.eq(&name))
        .select(schema::instructors::id)
        .get_result::<i32>(conn)
        .optional()?;

    match id {
        Some(id) => Ok(id),
        None => {
            let id = insert_into(schema::instructors::table)
                .values(NewInstructor { name })
                .returning(schema::instructors::id)
                .execute(conn)?;
            Ok(id as i32)
        }
    }
}
