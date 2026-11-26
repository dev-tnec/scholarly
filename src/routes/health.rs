use axum::extract::State;
use axum::http::HeaderValue;
use axum::response::{IntoResponse, Response};

use super::AppState;

/// 200 with a database ping, PRD section 6.
#[axum::debug_handler]
pub async fn health(State(state): State<AppState>) -> Response {
    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
        .is_ok();
    let body = if db_ok { "ok" } else { "db unreachable" };
    let status = if db_ok {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        [
            (
                axum::http::header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            ),
            (
                axum::http::header::CACHE_CONTROL,
                HeaderValue::from_static("no-store"),
            ),
        ],
        body,
    )
        .into_response()
}
