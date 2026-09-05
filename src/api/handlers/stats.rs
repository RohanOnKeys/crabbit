use std::sync::Arc;

use axum::extract::State;
use axum::Json;

use crate::api::dto::StatsResponse;
use crate::indexer::manifest::segment_dir;
use crate::indexer::segment::SegmentReader;
use crate::state::EngineState;

pub async fn stats(State(state): State<Arc<EngineState>>) -> Json<StatsResponse> {
    let segment_ids = state.manifest.lock().unwrap().segments.clone();

    let doc_count: u64 = segment_ids
        .iter()
        .filter_map(|id| SegmentReader::open(&segment_dir(&state.data_dir, *id), *id).ok())
        .map(|reader| reader.doc_count() as u64)
        .sum();

    Json(StatsResponse {
        doc_count,
        segment_count: segment_ids.len() as u64,
        query_count: state.query_count(),
        avg_query_latency_micros: state.avg_query_latency_micros(),
    })
}
