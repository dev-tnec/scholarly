// Integration tests for the directory and detail routes.
// #[ignore]: these need a live Postgres. Run them by pointing DATABASE_URL at
// a Neon branch with the migrations and tests/fixtures/seed.sql applied:
//   cargo test -- --ignored
// Everything else in this crate passes offline.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use scholarly::config::Config;
use scholarly::routes::{AppState, router};
use tower::ServiceExt;

async fn test_state() -> AppState {
    let config = Config::from_env().expect("DATABASE_URL must point at a seeded Neon branch");
    AppState::connect(config).await.expect("database connect")
}

async fn get(state: &AppState, path: &str) -> (StatusCode, String) {
    let app = router(state.clone());
    let response = app
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
#[ignore]
async fn directory_root_is_indexable() {
    let state = test_state().await;
    let (status, body) = get(&state, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains(r#"<link rel="canonical" href="https://"#));
    assert!(!body.contains("noindex"));
    assert!(body.contains("<h1"));
    assert!(body.contains("NL Scholarship"), "seeded card missing");
}

#[tokio::test]
#[ignore]
async fn filtered_directory_is_noindex_with_root_canonical() {
    let state = test_state().await;
    let (status, body) = get(&state, "/?jenjang=S1&sort=deadline").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("noindex, follow"));
    assert!(body.contains(r#"<link rel="canonical" href="https://"#));
}

#[tokio::test]
#[ignore]
async fn detail_page_renders_sections_and_trailing_slash_canonical() {
    let state = test_state().await;
    let (status, body) = get(&state, "/nl-scholarship/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<h1>NL Scholarship</h1>"));
    assert!(body.contains("Ringkasan</h2>"));
    assert!(body.contains("Sumber Resmi"));
    assert!(body.contains("nl-scholarship/"));
    assert!(body.contains("Buka Situs Resmi"));
    assert!(body.contains("Tanyakan ke Minol") || body.contains("Minol"));
}

#[tokio::test]
#[ignore]
async fn unknown_slug_is_404() {
    let state = test_state().await;
    let (status, body) = get(&state, "/tidak-ada-beasiswa/").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("Halaman tidak ditemukan"));
}

#[tokio::test]
#[ignore]
async fn sitemap_lists_published_set() {
    let state = test_state().await;
    let (status, body) = get(&state, "/sitemap.xml").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("/nl-scholarship/"));
    assert!(body.contains("<lastmod>"));
}

#[tokio::test]
#[ignore]
async fn robots_points_at_sitemap() {
    let state = test_state().await;
    let (status, body) = get(&state, "/robots.txt").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Sitemap: "));
    assert!(body.contains("/sitemap.xml"));
}

#[tokio::test]
#[ignore]
async fn healthz_pings_database() {
    let state = test_state().await;
    let (status, body) = get(&state, "/healthz").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "ok");
}

#[tokio::test]
#[ignore]
async fn html_budget_under_100kb() {
    let state = test_state().await;
    let (_, body) = get(&state, "/").await;
    assert!(
        body.len() < 100 * 1024,
        "directory HTML is {} bytes, budget is 102400",
        body.len()
    );
    let (_, detail) = get(&state, "/nl-scholarship/").await;
    assert!(
        detail.len() < 100 * 1024,
        "detail HTML is {} bytes, budget is 102400",
        detail.len()
    );
}

#[tokio::test]
#[ignore]
async fn content_type_is_html() {
    let state = test_state().await;
    let app = router(state.clone());
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/html; charset=utf-8"
    );
}
