use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResultItem {
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub score: f32,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResultItem>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize)]
pub struct IndexRequest {
    pub url: String,
    pub title: Option<String>,
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct IndexResponse {
    pub doc_id: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub doc_count: u64,
    pub segment_count: u64,
    pub query_count: u64,
    pub avg_query_latency_micros: u64,
}
