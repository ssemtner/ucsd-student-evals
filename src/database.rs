use crate::settings;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::{FromRow, Pool, Postgres};

// use crate::settings;
use anyhow::Result;
// use diesel::backend::Backend;
// use diesel::deserialize::FromSql;
// use diesel::prelude::*;
// use diesel::serialize::{IsNull, Output, ToSql};
// use diesel::sql_types::Text;
// use diesel::{deserialize, serialize, AsExpression, FromSqlRow};
// use diesel::pg::Pg;
// use serde::{Deserialize, Serialize};
//
pub async fn establish_connection() -> Result<Pool<Postgres>, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&settings().database_url)
        .await
}
//
// #[derive(Queryable, Selectable, Insertable, Identifiable, Debug, PartialEq, Clone)]
// #[diesel(table_name = crate::schema::units)]
#[derive(Debug, PartialEq)]
pub struct Unit {
    pub id: i32,
    pub name: String,
}
//
// #[derive(
//     Queryable,
//     Selectable,
//     Insertable,
//     Identifiable,
//     Associations,
//     serde::Serialize,
//     Debug,
//     PartialEq,
//     Clone,
// )]
// #[diesel(primary_key(code))]
// #[diesel(belongs_to(Unit))]
// #[diesel(table_name = crate::schema::courses)]
#[derive(FromRow, Debug, PartialEq)]
pub struct Course {
    pub code: String,
    pub name: String,
    pub unit_id: i32,
}
//
// #[derive(
//     Queryable, Selectable, Insertable, Identifiable, Associations, Debug, PartialEq, Clone,
// )]
// #[diesel(primary_key(sid))]
// #[diesel(belongs_to(Course, foreign_key = course_code))]
// #[diesel(table_name = crate::schema::sids)]
#[derive(FromRow, Debug, PartialEq)]
pub struct SectionId {
    pub sid: i32,
    pub course_code: String,
}

// #[derive(Debug, AsExpression, Deserialize, Serialize, FromSqlRow, PartialEq)]
// #[diesel(sql_type = Text)]
// pub struct JsonArray(serde_json::Value);
//
// impl JsonArray {
//     pub fn new<const N: usize>(arr: [u32; N]) -> Self {
//         let values = arr
//             .into_iter()
//             .map(|x| serde_json::to_value(x).unwrap())
//             .collect();
//         Self(serde_json::Value::Array(values))
//     }
//
//     pub fn into_vec(self) -> Vec<u32> {
//         let values = self.0;
//         values
//             .as_array()
//             .unwrap()
//             .iter()
//             .map(|val| val.as_u64().unwrap() as u32)
//             .collect()
//     }
// }
//
// impl<const N: usize> From<[u32; N]> for JsonArray {
//     fn from(value: [u32; N]) -> Self {
//         Self::new(value)
//     }
// }
//
// impl<DB> FromSql<Text, DB> for JsonArray
// where
//     DB: Backend,
//     String: FromSql<Text, DB>,
// {
//     fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
//         let t = <String as FromSql<Text, DB>>::from_sql(bytes)?;
//         Ok(Self(serde_json::from_str(&t)?))
//     }
// }
//
// impl ToSql<Text, Pg> for JsonArray
// where
//     String: ToSql<Text, Pg>,
// {
//     fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
//         let s = serde_json::to_string(&self.0)?;
//         // out.set_value(s.as_bytes());
//         Ok(IsNull::No)
//     }
// }
//
// #[derive(Queryable, Insertable, Selectable, Identifiable, Serialize, Debug, PartialEq)]
// #[diesel(primary_key(sid))]
// #[diesel(table_name = crate::schema::evaluations)]
pub struct Evaluation {
    pub sid: i32,
    pub section_name: String,
    pub course_code: String,
    pub term_id: i32,
    pub instructor_id: i32,

    pub enrollment: i32,
    pub responses: i32,

    pub class_helped_understanding: Value,
    pub assignments_helped_understanding: Value,
    pub fair_exams: Value,
    pub timely_feedback: Value,
    pub developed_understanding: Value,
    pub engaging: Value,
    pub communication: Value,
    pub help_opportunities: Value,
    pub effective_methods: Value,
    pub timeliness: Value,
    pub welcoming: Value,
    pub materials: Value,
    pub hours: Value,
    pub expected_grades: Value,
    pub actual_grades: Value,
}
//
// #[derive(Queryable, Selectable, Identifiable, Debug)]
// #[diesel(table_name = crate::schema::terms)]
// pub struct Term {
//     pub id: i32,
//     pub name: String,
// }
//
// #[derive(Insertable, Debug)]
// #[diesel(table_name = crate::schema::terms)]
// pub struct NewTerm {
//     pub name: String,
// }
//
// #[derive(Queryable, Selectable, Identifiable, Serialize, Debug)]
// #[diesel(table_name = crate::schema::instructors)]
// pub struct Instructor {
//     pub id: i32,
//     pub name: String,
// }
//
// #[derive(Insertable, Debug)]
// #[diesel(table_name = crate::schema::instructors)]
// pub struct NewInstructor {
//     pub name: String,
// }
