use askama::Template;
use axum::extract::State;
use axum::http::HeaderValue;
use axum::response::{IntoResponse, Response};
use chrono::NaiveDate;

use super::{AppState, server_error};
use crate::domain::scholarship::detail_path;

#[derive(sqlx::FromRow)]
struct SitemapRow {
    slug: String,
    checked_at: NaiveDate,
}

#[derive(Template)]
#[template(path = "sitemap.xml", escape = "none")]
struct SitemapView {
    base_url: String,
    entries: Vec<SitemapEntry>,
}

struct SitemapEntry {
    url: String,
    lastmod: String,
}

pub async fn sitemap(State(state): State<AppState>) -> Response {
    let rows: Result<Vec<SitemapRow>, _> = sqlx::query_as(
        "SELECT slug, checked_at FROM scholarships WHERE published ORDER BY checked_at DESC, slug",
    )
    .fetch_all(&state.pool)
    .await;
    let rows = match rows {
        Ok(rows) => rows,
        Err(e) => return server_error("sitemap", e),
    };
    let base = &state.config.base_url;
    let view = SitemapView {
        base_url: base.to_string(),
        entries: rows
            .into_iter()
            .map(|r| SitemapEntry {
                url: format!("{base}{}", detail_path(&r.slug)),
                lastmod: r.checked_at.format("%Y-%m-%d").to_string(),
            })
            .collect(),
    };
    let body = match view.render() {
        Ok(body) => body,
        Err(e) => {
            tracing::error!("sitemap render: {e}");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    (
        [
            (
                axum::http::header::CONTENT_TYPE,
                HeaderValue::from_static("application/xml; charset=utf-8"),
            ),
            (
                axum::http::header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=3600"),
            ),
        ],
        body,
    )
        .into_response()
}

pub async fn robots(State(state): State<AppState>) -> Response {
    let body = format!(
        "User-agent: *\nAllow: /\nDisallow: /healthz\nDisallow: /*?\n\nSitemap: {}/sitemap.xml\n",
        state.config.base_url
    );
    (
        [
            (
                axum::http::header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            ),
            (
                axum::http::header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=3600"),
            ),
        ],
        body,
    )
        .into_response()
}
