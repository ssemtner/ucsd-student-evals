use crate::api::internal_error;
use crate::schema::{courses, units};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use deadpool_diesel::postgres::Pool;
use diesel::{QueryDsl, Queryable, RunQueryDsl, TextExpressionMethods};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub fn get_router() -> Router<Pool> {
    Router::new().route("/", get(search))
}

#[derive(Deserialize)]
struct SearchQuery {
    page: Option<usize>,
    per_page: Option<usize>,
    q: Option<String>,
}

#[derive(Queryable, Serialize)]
struct SearchResult {
    code: String,
    name: String,
    unit: String,
}

async fn search(
    State(pool): State<Pool>,
    args: Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;
    let per_page = args.per_page.unwrap_or(20).min(100);
    let res = conn
        .interact(move |conn| {
            let like = format!("%{}%", args.q.clone().unwrap_or_default());
            courses::table
                .inner_join(units::table)
                .select((courses::code, courses::name, units::name))
                .filter(courses::name.like(&like))
                .offset(
                    args.page
                        .map(|page| (page.max(1) - 1) * per_page)
                        .unwrap_or_default() as i64,
                )
                .limit(per_page as i64)
                .get_results::<SearchResult>(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;
    Ok(Json(json!(res)))
}
