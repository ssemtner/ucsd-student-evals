mod auth;
mod courses;
mod evaluations;

use anyhow::Result;
use auth::authorization_middleware;
use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use axum::Router;
use sqlx::{Pool, Postgres};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

pub fn app(pool: Pool<Postgres>) -> Result<Router> {
    let router = Router::new()
        .nest("/v1/courses", courses::get_router())
        .nest("/v1/evals", evaluations::get_router())
        .route_layer(from_fn_with_state(pool.clone(), authorization_middleware))
        .route("/v1", get(root))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(pool);

    Ok(router)
}

async fn root() -> &'static str {
    "ucsd-student-evals API Version 1.0.0\n"
}

fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
