use crate::api::internal_error;
use crate::database::Instructor;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Serialize, Serializer};
use serde_json::json;
use sqlx::{query, query_as, Pool, Postgres};
use std::collections::HashMap;

pub fn get_router() -> Router<Pool<Postgres>> {
    Router::new()
        .route("/sid/:sid", get(eval_summary))
        .route("/:code", get(summary))
        .route("/:code/instructors", get(instructors))
        .route("/:code/sections", get(list_evals))
}

async fn instructors(
    Path(code): Path<String>,
    State(pool): State<Pool<Postgres>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let res = query_as!(
        Instructor,
        "
            SELECT DISTINCT id, name FROM instructors
            INNER JOIN evaluations ON evaluations.instructor_id = instructors.id
            WHERE evaluations.course_code ILIKE $1
        ",
        code
    )
    .fetch_all(&pool)
    .await
    .map_err(internal_error)?;
    Ok(Json(json!(res)))
}

async fn list_evals(
    Path(code): Path<String>,
    State(pool): State<Pool<Postgres>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let res = query!(
        "SELECT sid FROM evaluations WHERE course_code ILIKE $1",
        code
    )
    .fetch_all(&pool)
    .await
    .map_err(internal_error)?
    .into_iter()
    .map(|row| row.sid)
    .collect::<Vec<_>>();

    Ok(Json(json!(res)))
}

#[derive(Serialize, Debug)]
struct Summary {
    sections: i64,
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
    State(pool): State<Pool<Postgres>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let res = query!(
        "
            WITH stats AS (
                SELECT
                    instructors.id,
                    instructors.name,
                    (
                        SELECT (SUM(n * w) / SUM(CASE WHEN i <= 5 THEN n ELSE 0 END))::float8
                        FROM UNNEST(actual_grades, array[4.0, 3.0, 2.0, 1.0]) WITH ORDINALITY AS arr(n, w, i)
                    ) AS actual_gpa,
                    (
                        SELECT (SUM(n * w) / SUM(CASE WHEN i <= 5 THEN n ELSE 0 END))::float8
                        FROM UNNEST(expected_grades, array[4.0, 3.0, 2.0, 1.0]) WITH ORDINALITY AS arr(n, w, i)
                    ) AS expected_gpa,
                    (
                        SELECT (SUM(n * w) / SUM(n))::float8
                        FROM UNNEST(hours, CASE
                            WHEN CARDINALITY(hours) = 4 THEN array[0.0, 5.0, 10.0, 15.0]
                            ELSE array[1.0, 3.0, 5.0, 7.0, 9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0]
                        END)
                        WITH ORDINALITY AS arr(n, w, i)
                    ) AS hours
                FROM evaluations
                INNER JOIN instructors ON evaluations.instructor_id = instructors.id
                WHERE course_code = $1
            )

            SELECT
                name as \"instructor!\",
                COUNT(*) AS \"sections!: i64\",
                COALESCE(AVG(actual_gpa), -1.0) AS \"actual_gpa!: f64\",
                COALESCE(AVG(expected_gpa), -1.0) AS \"expected_gpa!: f64\",
                COALESCE(AVG(hours), -1.0) AS \"hours!: f64\"
            FROM stats
            GROUP BY id, name

            UNION ALL

            SELECT
                'overall' as \"instructor!\",
                COUNT(*) AS \"sections!: i64\",
                COALESCE(AVG(actual_gpa), -1.0) AS \"actual_gpa!: f64\",
                COALESCE(AVG(expected_gpa), -1.0) AS \"expected_gpa!: f64\",
                COALESCE(AVG(hours), -1.0) AS \"hours!: f64\"
            FROM stats
        ",
        code
    ).fetch_all(&pool).await.map_err(internal_error)?
        .into_iter()
        .map(|row| {
            (row.instructor, Summary {
                sections: row.sections,
                actual_gpa: row.actual_gpa,
                expected_gpa: row.expected_gpa,
                hours: row.hours,
            })
        })
        .collect::<HashMap<_, _>>();

    Ok(Json(json!(res)))
}

async fn eval_summary(
    Path(sid): Path<i32>,
    State(pool): State<Pool<Postgres>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let res = query_as!(Summary,
        "
            SELECT
                1 AS \"sections!: i64\",
                COALESCE((
                    SELECT (SUM(n * w) / SUM(CASE WHEN i <= 5 THEN n ELSE 0 END))::float8
                    FROM UNNEST(actual_grades, array[4.0, 3.0, 2.0, 1.0]) WITH ORDINALITY AS arr(n, w, i)
                ), -1.0) AS \"actual_gpa!: f64\",
                COALESCE((
                    SELECT (SUM(n * w) / SUM(CASE WHEN i <= 5 THEN n ELSE 0 END))::float8
                    FROM UNNEST(expected_grades, array[4.0, 3.0, 2.0, 1.0]) WITH ORDINALITY AS arr(n, w, i)
                ), -1.0) AS \"expected_gpa!: f64\",
                COALESCE((
                    SELECT (SUM(n * w) / SUM(n))::float8
                    FROM UNNEST(hours, CASE
                        WHEN CARDINALITY(hours) = 4 THEN array[0.0, 5.0, 10.0, 15.0]
                        ELSE array[1.0, 3.0, 5.0, 7.0, 9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0]
                    END)
                    WITH ORDINALITY AS arr(n, w, i)
                ), -1.0) AS \"hours!: f64\"
            FROM evaluations
            WHERE sid = $1
        ",
        sid
    ).fetch_one(&pool).await.map_err(internal_error)?;

    Ok(Json(json!(res)))
}
