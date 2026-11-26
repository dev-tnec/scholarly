mod detail;
mod directory;
mod health;
mod sitemap;

use std::collections::HashMap;
use std::hash::Hasher;
use std::path::PathBuf;
use std::sync::Arc;

use askama::Template;
use axum::extract::State;
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use sqlx::PgPool;
use tokio::sync::RwLock;
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;

use crate::config::Config;
use crate::db;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
    redirects: Arc<RwLock<HashMap<String, (String, u16)>>>,
}

impl AppState {
    pub async fn connect(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = db::connect(&config.database_url).await?;
        db::migrate(&pool).await?;
        let state = Self {
            pool,
            config,
            redirects: Arc::new(RwLock::new(HashMap::new())),
        };
        state.reload_redirects().await;
        Ok(state)
    }

    pub async fn reload_redirects(&self) {
        let rows: Result<Vec<(String, String, i32)>, _> =
            sqlx::query_as("SELECT from_path, to_path, status_code FROM redirects")
                .fetch_all(&self.pool)
                .await;
        match rows {
            Ok(list) => {
                *self.redirects.write().await = list
                    .into_iter()
                    .map(|(from, to, code)| (from, (to, code as u16)))
                    .collect();
            }
            Err(e) => tracing::warn!("redirects load failed: {e}"),
        }
    }
}

/// Rendered HTML with the PRD section 10 response headers.
pub struct Html<T: Template>(pub T);

impl<T: Template> IntoResponse for Html<T> {
    fn into_response(self) -> Response {
        let body = match self.0.render() {
            Ok(body) => body,
            Err(e) => {
                tracing::error!("template render failed: {e}");
                return (StatusCode::INTERNAL_SERVER_ERROR, "kesalahan server").into_response();
            }
        };
        // shortcut: weak hash ETag, fine for revalidation, upgrade to sha256 if collisions appear
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash_slice(body.as_bytes(), &mut hasher);
        let etag = format!("W/\"{:x}-{:x}\"", hasher.finish(), body.len());
        (
            [
                (
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("text/html; charset=utf-8"),
                ),
                (header::ETAG, HeaderValue::from_str(&etag).unwrap()),
                (
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=60, stale-while-revalidate=300"),
                ),
            ],
            body,
        )
            .into_response()
    }
}

#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundView {
    base_url: String,
}

async fn not_found() -> Response {
    not_found_response()
}

/// Shared 404 body, PRD section 6: unknown or unpublished slugs get the
/// designed 404 page with a 404 status, never a stack trace.
pub fn not_found_response() -> Response {
    let mut response = Html(NotFoundView {
        base_url: "https://scholarly.titiknolenglish.com".into(),
    })
    .into_response();
    *response.status_mut() = StatusCode::NOT_FOUND;
    response
}

async fn redirect_middleware(
    State(state): State<AppState>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    if let Some((to, code)) = state.redirects.read().await.get(&path) {
        let mut response = Response::new(axum::body::Body::empty());
        *response.status_mut() =
            StatusCode::from_u16(*code).unwrap_or(StatusCode::MOVED_PERMANENTLY);
        response
            .headers_mut()
            .insert(header::LOCATION, HeaderValue::from_str(to).unwrap());
        return response;
    }
    next.run(request).await
}

pub async fn serve(state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", state.config.bind, state.config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://{addr}");
    axum::serve(listener, router(state)).await?;
    Ok(())
}

pub fn router(state: AppState) -> Router {
    let static_dir = PathBuf::from("static");
    Router::new()
        .route("/", get(directory::page))
        .route("/sitemap.xml", get(sitemap::sitemap))
        .route("/robots.txt", get(sitemap::robots))
        .route("/healthz", get(health::health))
        .route("/{slug}", get(detail::page))
        .route("/{slug}/", get(detail::page))
        .fallback(not_found)
        .nest_service("/static", ServeDir::new(static_dir))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            redirect_middleware,
        ))
        .layer(security_headers())
        .layer(cache_control_defaults())
        .layer(CompressionLayer::new())
        .with_state(state)
}

fn security_headers() -> tower_http::set_header::SetResponseHeaderLayer<HeaderValue> {
    let csp = "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data: https:; \
               font-src 'self'; connect-src 'self'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'";
    tower_http::set_header::SetResponseHeaderLayer::overriding(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(csp),
    )
}

fn cache_control_defaults() -> tower_http::set_header::SetResponseHeaderLayer<HeaderValue> {
    tower_http::set_header::SetResponseHeaderLayer::if_not_present(
        header::CACHE_CONTROL,
        // shortcut: no content hash yet, 5 min cache keeps manual edits visible; add hashed assets at deploy hardening
        HeaderValue::from_static("public, max-age=300"),
    )
}

/// Small JSON error helper so handlers never leak internals.
pub fn server_error(context: &str, e: sqlx::Error) -> Response {
    tracing::error!("{context}: {e}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": "kesalahan server" })),
    )
        .into_response()
}
