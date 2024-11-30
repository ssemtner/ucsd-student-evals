use crate::api::internal_error;
use crate::database::Instructor;
use crate::schema::{evaluations, instructors};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use deadpool_diesel::sqlite::Pool;
use diesel::dsl::{avg, case_when, sql};
use diesel::expression::SqlLiteral;
use diesel::sql_types::{Double, Text};
use diesel::{
    define_sql_function, Column, CombineDsl, ExpressionMethods, NullableExpressionMethods,
    QueryDsl, Queryable, RunQueryDsl, SelectableHelper, TextExpressionMethods,
};
use serde::{Serialize, Serializer};
use serde_json::json;
use std::collections::HashMap;

pub fn get_router() -> Router<Pool> {
    Router::new()
        .route("/sid/:sid", get(eval_summary))
        .route("/:code", get(summary))
        .route("/:code/instructors", get(instructors))
        .route("/:code/evals", get(list_evals))
}

async fn instructors(
    Path(code): Path<String>,
    State(pool): State<Pool>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;
    let res = conn
        .interact(|conn| {
            instructors::table
                .inner_join(evaluations::table)
                .filter(evaluations::course_code.like(code))
                .select(Instructor::as_select())
                .distinct()
                .get_results::<Instructor>(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(Json(json!(res)))
}

async fn list_evals(
    Path(code): Path<String>,
    State(pool): State<Pool>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;
    let res = conn
        .interact(|conn| {
            evaluations::table
                .select(evaluations::sid)
                .filter(evaluations::course_code.like(code))
                .get_results::<i32>(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(Json(json!(res)))
}

#[derive(Queryable, Serialize)]
struct Summary {
    #[serde(rename = "actualGPA", serialize_with = "float_as_str")]
    actual_gpa: f64,
    #[serde(rename = "expectedGPA", serialize_with = "float_as_str")]
    expected_gpa: f64,
    #[serde(serialize_with = "float_as_str")]
    hours: f64,
}

fn float_as_str<S>(x: &f64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let rounded = format!("{:.2}", x);
    s.serialize_str(&rounded)
}

async fn summary(
    Path(code): Path<String>,
    State(pool): State<Pool>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;

    let res = conn
        .interact(move |conn| {
            let q = evaluations::table.filter(evaluations::course_code.like(code));
            let actual_gpa_select = avg(extract_mean(evaluations::actual_grades, 5, |i| 4 - i));
            let expected_gpa_select = avg(extract_mean(evaluations::expected_grades, 5, |i| 4 - i));
            let hours_select = avg(case_when(
                json_array_length(evaluations::hours).eq(4),
                extract_mean(evaluations::hours, 4, |i| i * 5),
            )
            .otherwise(extract_mean(evaluations::hours, 11, |i| i * 2 + 1)));
            q.clone()
                .inner_join(instructors::table)
                .group_by(instructors::id)
                .select((
                    instructors::name.nullable(),
                    actual_gpa_select.clone(),
                    expected_gpa_select.clone(),
                    hours_select.clone(),
                ))
                .union_all(q.select((
                    sql::<Text>("'overall'").nullable(),
                    actual_gpa_select,
                    expected_gpa_select,
                    hours_select,
                )))
                .get_results::<(Option<String>, Option<f64>, Option<f64>, Option<f64>)>(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;
    let res: HashMap<String, Summary> = res
        .into_iter()
        .filter_map(|(a, b, c, d)| a.zip(b).zip(c).zip(d).map(|(((a, b), c), d)| (a, b, c, d)))
        .map(|(instructor, actual_gpa, expected_gpa, hours)| {
            (
                instructor,
                Summary {
                    actual_gpa,
                    expected_gpa,
                    hours,
                },
            )
        })
        .collect();
    Ok(Json(json!(res)))
}

async fn eval_summary(
    Path(sid): Path<i32>,
    State(pool): State<Pool>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;

    let res = conn
        .interact(move |conn| {
            evaluations::table
                .filter(evaluations::sid.eq(sid))
                .select((
                    extract_mean(evaluations::actual_grades, 5, |i| 4 - i),
                    extract_mean(evaluations::expected_grades, 5, |i| 4 - i),
                    case_when(
                        json_array_length(evaluations::hours).eq(4),
                        extract_mean(evaluations::hours, 4, |i| i * 5),
                    )
                    .otherwise(extract_mean(evaluations::hours, 11, |i| i * 2 + 1)),
                ))
                .get_result::<Summary>(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;
    Ok(Json(json!(res)))
}

fn column_name<T>(_: T) -> &'static str
where
    T: Column,
{
    T::NAME
}

define_sql_function! {
    fn json_array_length(a: Text) -> Integer
}

fn extract_mean<Col>(column: Col, size: u32, weight: fn(u32) -> u32) -> SqlLiteral<Double>
where
    Col: Column,
{
    let col = column_name(column);
    let numerator = (0..size)
        .map(|i| format!("json_extract({col}, '$[{i}]') * {}.0", weight(i)))
        .collect::<Vec<_>>()
        .join("+");
    let denominator = (0..size)
        .map(|i| format!("json_extract({col}, '$[{i}]')"))
        .collect::<Vec<_>>()
        .join("+");
    sql(&format!("({}) / ({})", numerator, denominator))
}
