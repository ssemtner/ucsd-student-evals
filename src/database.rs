use crate::settings;
use anyhow::Result;
use diesel::prelude::*;

pub fn establish_connection() -> Result<SqliteConnection> {
    SqliteConnection::establish(&settings().database_url).map_err(|e| e.into())
}

#[derive(Queryable, Selectable, Insertable, Identifiable, Debug, PartialEq)]
#[diesel(table_name = crate::schema::units)]
pub struct Unit {
    pub id: i32,
    pub name: String,
}

#[derive(Queryable, Selectable, Insertable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(code))]
#[diesel(belongs_to(Unit))]
#[diesel(table_name = crate::schema::courses)]
pub struct Course {
    pub code: String,
    pub name: String,
    pub unit_id: i32,
}
