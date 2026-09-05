use std::collections::HashSet;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use crabbit::api::routes::build_router;
use crabbit::config::{Config, CrawlerConfig, IndexConfig, PaginationConfig, ServerConfig};
use crabbit::state::EngineState;

fn test_config(data_dir: &std::path::Path) -> Config {
    Config {
        server: ServerConfig { host: "127.0.0.1".into(), port: 0 },
        index: IndexConfig { shard_count: 1, data_dir: data_dir.to_string_lossy().into_owned() },
        pagination: PaginationConfig {
            default_page_size: 20,
            max_page_size: 100,
            cursor_ttl_seconds: 3600,
        },
        crawler: CrawlerConfig { concurrency: 1, user_agent: "test".into(), seed_urls: vec![] },
    }
}

async fn post_json(router: &axum::Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::post(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    (status, value)
}

async fn get_json(router: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::get(uri).body(Body::empty()).unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    (status, value)
}

#[tokio::test]
async fn index_and_search_round_trip_with_pagination() {
    let tmp = tempfile::tempdir().unwrap();
    let state = Arc::new(EngineState::new(test_config(tmp.path())).unwrap());
    let router = build_router(state);

    let docs = [
        ("https://example.com/a", "Doc A", "rust web frameworks are fast"),
        ("https://example.com/b", "Doc B", "rust search engines rank documents"),
        ("https://example.com/c", "Doc C", "totally unrelated content about cooking"),
    ];
    for (url, title, body) in docs {
        let (status, _) = post_json(
            &router,
            "/index",
            json!({ "url": url, "title": title, "body": body }),
        )
        .await;
        assert_eq!(status, StatusCode::ACCEPTED);
    }

    let (status, page1) = get_json(&router, "/search?q=rust&limit=1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page1["results"].as_array().unwrap().len(), 1);
    assert_eq!(page1["has_more"], true);
    let cursor = page1["next_cursor"].as_str().unwrap();

    let (status, page2) =
        get_json(&router, &format!("/search?q=rust&limit=1&cursor={cursor}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page2["has_more"], false);

    let urls: HashSet<String> = [&page1, &page2]
        .iter()
        .flat_map(|p| p["results"].as_array().unwrap())
        .map(|r| r["url"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(urls.len(), 2);
    assert!(urls.contains("https://example.com/a"));
    assert!(urls.contains("https://example.com/b"));

    let (status, unrelated) = get_json(&router, "/search?q=cooking&limit=10").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(unrelated["results"].as_array().unwrap().len(), 1);
    assert_eq!(unrelated["results"][0]["url"], "https://example.com/c");
}

#[tokio::test]
async fn stats_reflects_indexed_docs() {
    let tmp = tempfile::tempdir().unwrap();
    let state = Arc::new(EngineState::new(test_config(tmp.path())).unwrap());
    let router = build_router(state);

    post_json(&router, "/index", json!({ "url": "u1", "title": "t1", "body": "hello world" }))
        .await;
    post_json(&router, "/index", json!({ "url": "u2", "title": "t2", "body": "hello again" }))
        .await;

    let (status, stats) = get_json(&router, "/stats").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(stats["doc_count"], 2);
}

#[tokio::test]
async fn bad_cursor_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let state = Arc::new(EngineState::new(test_config(tmp.path())).unwrap());
    let router = build_router(state);

    let req = Request::get("/search?q=x&cursor=not-valid-base64!!")
        .body(Body::empty())
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
