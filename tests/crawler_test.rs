use std::sync::Arc;

use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crabbit::config::{Config, CrawlerConfig, IndexConfig, PaginationConfig, ServerConfig};
use crabbit::crawler::worker::run;
use crabbit::indexer::manifest::{manifest_path, Manifest};
use crabbit::state::EngineState;

fn test_config(data_dir: &std::path::Path, seed_urls: Vec<String>) -> Config {
    Config {
        server: ServerConfig { host: "127.0.0.1".into(), port: 0 },
        index: IndexConfig { shard_count: 1, data_dir: data_dir.to_string_lossy().into_owned() },
        pagination: PaginationConfig {
            default_page_size: 20,
            max_page_size: 100,
            cursor_ttl_seconds: 3600,
        },
        crawler: CrawlerConfig { concurrency: 4, user_agent: "CrabbitBot/1.0".into(), seed_urls },
    }
}

#[tokio::test]
async fn crawls_seed_urls_and_dedupes_identical_content() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/page-a"))
        .and(header("user-agent", "CrabbitBot/1.0"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "<html><head><title>Page A</title></head><body>rust search engine crawler</body></html>",
        ))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/page-b"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "<html><head><title>Page B</title></head><body>rust web framework guide</body></html>",
        ))
        .mount(&server)
        .await;

    // Duplicate of page-a's body under a different URL: should be skipped by dedup.
    Mock::given(method("GET"))
        .and(path("/page-a-mirror"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "<html><head><title>Page A</title></head><body>rust search engine crawler</body></html>",
        ))
        .mount(&server)
        .await;

    let tmp = tempfile::tempdir().unwrap();
    let seeds = vec![
        format!("{}/page-a", server.uri()),
        format!("{}/page-b", server.uri()),
        format!("{}/page-a-mirror", server.uri()),
    ];
    let state = Arc::new(EngineState::new(test_config(tmp.path(), seeds)).unwrap());

    run(state.clone()).await.unwrap();

    let manifest = Manifest::load(&manifest_path(&state.data_dir)).unwrap();
    assert_eq!(manifest.segments.len(), 2, "expected 2 indexed docs after dedup");
}

#[tokio::test]
async fn empty_seed_list_indexes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let state = Arc::new(EngineState::new(test_config(tmp.path(), vec![])).unwrap());

    run(state.clone()).await.unwrap();

    let manifest = Manifest::load(&manifest_path(&state.data_dir)).unwrap();
    assert!(manifest.segments.is_empty());
}
