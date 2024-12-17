use std::fmt::Debug;

use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use sqlx::{Pool, Postgres};

pub async fn authorization_middleware(
    State(pool): State<Pool<Postgres>>,
    header: Option<TypedHeader<Authorization<Bearer>>>,
    code: Option<Path<String>>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let Some(TypedHeader(Authorization(bearer))) = header else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Missing Authorization header".to_string(),
        ));
    };
    let token = bearer.token();

    let token_id = sqlx::query!("SELECT id FROM tokens WHERE token LIKE $1", token)
        .fetch_one(&pool)
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token".to_string()))?
        .id;

    if let Some(Path(code)) = code {
        let _ = sqlx::query!(
            "
                INSERT INTO course_accesses (num, token_id, course_code)
                VALUES (1, $1, $2)
                ON CONFLICT (course_code) DO UPDATE
                SET num = course_accesses.num + 1
            ",
            token_id,
            code,
        )
        .execute(&pool)
        .await;
    }

    let response = next.run(request).await;

    Ok(response)
}
