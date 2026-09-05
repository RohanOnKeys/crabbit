use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::api::dto::{SearchParams, SearchResponse, SearchResultItem};
use crate::indexer::manifest::segment_dir;
use crate::indexer::segment::SegmentReader;
use crate::pagination::Cursor;
use crate::query::executor;
use crate::state::EngineState;

pub async fn search(
    State(state): State<Arc<EngineState>>,
    Query(params): Query<SearchParams>,
) -> Response {
    let started = Instant::now();
    let page_cfg = &state.config.pagination;

    let limit = params
        .limit
        .unwrap_or(page_cfg.default_page_size)
        .clamp(1, page_cfg.max_page_size);

    let now = crate::api::unix_now();
    let cursor = match params.cursor.as_deref() {
        Some(raw) => match Cursor::decode(raw, now, page_cfg.cursor_ttl_seconds) {
            Ok(c) => Some(c),
            Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        },
        None => None,
    };

    let segment_ids = state.manifest.lock().unwrap().segments.clone();
    let readers: Vec<SegmentReader> = segment_ids
        .into_iter()
        .filter_map(|id| SegmentReader::open(&segment_dir(&state.data_dir, id), id).ok())
        .collect();

    let page = executor::search(&readers, &params.q, limit, cursor, &state.pipeline, now);

    let results = page
        .results
        .into_iter()
        .map(|r| SearchResultItem {
            url: r.meta.url,
            title: r.meta.title,
            snippet: r.meta.snippet,
            score: r.score,
        })
        .collect();

    state.record_query(started.elapsed().as_micros() as u64);

    Json(SearchResponse {
        results,
        next_cursor: page.next_cursor.map(|c| c.encode()),
        has_more: page.has_more,
    })
    .into_response()
}
