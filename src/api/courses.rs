use crate::api::internal_error;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{query_as, Pool, Postgres};

pub fn get_router() -> Router<Pool<Postgres>> {
    Router::new().route("/", get(search))
}

#[derive(Deserialize)]
struct SearchQuery {
    page: Option<i32>,
    per_page: Option<i32>,
    q: Option<String>,
}

#[derive(Serialize)]
struct SearchResult {
    code: String,
    name: String,
    unit: String,
}

async fn search(
    State(pool): State<Pool<Postgres>>,
    args: Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let per_page = args.per_page.unwrap_or(20).min(100);
    let like = format!("%{}%", args.q.clone().unwrap_or_default());
    let res = query_as!(SearchResult,
        "
            SELECT courses.code, courses.name, units.name as unit FROM courses
            INNER JOIN units ON courses.unit_id = units.id
            WHERE courses.name ILIKE $1
            ORDER BY
                SPLIT_PART(courses.name, ' ', 1),
                CAST(REGEXP_REPLACE(SPLIT_PART(courses.name, ' ', 2), '[A-Za-z]+$', '') AS INTEGER),
                NULLIF(REGEXP_REPLACE(SPLIT_PART(courses.name, ' ', 2), '^[0-9]+', ''), '') NULLS FIRST
            OFFSET $2 LIMIT $3
        ",
        like,
        args.page
            .map(|page| (page.max(1) - 1) * per_page)
            .unwrap_or_default() as i32,
        per_page as i32,
    )
    .fetch_all(&pool)
    .await
    .map_err(internal_error)?;

    Ok(Json(json!(res)))
}
