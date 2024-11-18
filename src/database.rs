use crate::settings;
use anyhow::Result;
use diesel::backend::Backend;
use diesel::deserialize::FromSql;
use diesel::prelude::*;
use diesel::serialize::{IsNull, Output, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;
use diesel::{deserialize, serialize, AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

pub fn establish_connection() -> Result<SqliteConnection> {
    SqliteConnection::establish(&settings().database_url).map_err(|e| e.into())
}

#[derive(Queryable, Selectable, Insertable, Identifiable, Debug, PartialEq, Clone)]
#[diesel(table_name = crate::schema::units)]
pub struct Unit {
    pub id: i32,
    pub name: String,
}

#[derive(
    Queryable, Selectable, Insertable, Identifiable, Associations, Debug, PartialEq, Clone,
)]
#[diesel(primary_key(code))]
#[diesel(belongs_to(Unit))]
#[diesel(table_name = crate::schema::courses)]
pub struct Course {
    pub code: String,
    pub name: String,
    pub unit_id: i32,
}

#[derive(Debug, AsExpression, Deserialize, Serialize, FromSqlRow, PartialEq)]
#[sql_type = "Text"]
pub struct JsonArray(serde_json::Value);

impl JsonArray {
    pub fn new<const N: usize>(arr: [u32; N]) -> Self {
        let values = arr
            .into_iter()
            .map(|x| serde_json::to_value(x).unwrap())
            .collect();
        Self(serde_json::Value::Array(values))
    }

    pub fn into_vec(self) -> Vec<u32> {
        let values = self.0;
        values
            .as_array()
            .unwrap()
            .iter()
            .map(|val| val.as_u64().unwrap() as u32)
            .collect()
    }
}

impl<const N: usize> From<[u32; N]> for JsonArray {
    fn from(value: [u32; N]) -> Self {
        Self::new(value)
    }
}

impl<const N: usize> Into<[u32; N]> for JsonArray {
    fn into(self) -> [u32; N] {
        self.into_vec().try_into().unwrap()
    }
}

impl<DB> FromSql<Text, DB> for JsonArray
where
    DB: Backend,
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let t = <String as FromSql<Text, DB>>::from_sql(bytes)?;
        Ok(Self(serde_json::from_str(&t)?))
    }
}

impl ToSql<Text, Sqlite> for JsonArray
where
    String: ToSql<Text, Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        let s = serde_json::to_string(&self.0)?;
        out.set_value(s);
        Ok(IsNull::No)
    }
}

#[derive(Queryable, Insertable, Selectable, Identifiable, Debug, PartialEq)]
#[diesel(primary_key(sid))]
#[diesel(table_name = crate::schema::evaluations)]
pub struct Evaluation {
    pub sid: i32,
    pub section_name: String,
    pub course_code: String,
    pub term_id: i32,
    pub instructor_id: i32,

    pub enrollment: i32,
    pub responses: i32,

    pub class_helped_understanding: JsonArray,
    pub assignments_helped_understanding: JsonArray,
    pub fair_exams: JsonArray,
    pub timely_feedback: JsonArray,
    pub developed_understanding: JsonArray,
    pub engaging: JsonArray,
    pub communication: JsonArray,
    pub help_opportunities: JsonArray,
    pub effective_methods: JsonArray,
    pub timeliness: JsonArray,
    pub welcoming: JsonArray,
    pub materials: JsonArray,
    pub hours: JsonArray,
    pub expected_grades: JsonArray,
    pub actual_grades: JsonArray,
}

#[derive(Queryable, Selectable, Identifiable, Debug)]
#[diesel(table_name = crate::schema::terms)]
pub struct Term {
    pub id: i32,
    pub name: String,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::schema::terms)]
pub struct NewTerm {
    pub name: String
}

#[derive(Queryable, Selectable, Identifiable, Debug)]
#[diesel(table_name = crate::schema::instructors)]
pub struct Instructor {
    pub id: i32,
    pub name: String,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::schema::instructors)]
pub struct NewInstructor {
    pub name: String
}
