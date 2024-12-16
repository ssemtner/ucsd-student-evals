mod courses;
mod evaluations;

use crate::settings;
use anyhow::Result;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use deadpool_diesel::sqlite::{Manager, Pool};
use deadpool_diesel::Runtime;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

pub fn app() -> Result<Router> {
    let manager = Manager::new(&settings().database_url, Runtime::Tokio1);
    let pool = Pool::builder(manager).build()?;

    let router = Router::new()
        .route("/v1", get(root))
        .nest("/v1/courses", courses::get_router())
        .nest("/v1/evals", evaluations::get_router())
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
