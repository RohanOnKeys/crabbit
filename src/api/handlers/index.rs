use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::api::dto::{IndexRequest, IndexResponse};
use crate::indexer::ingest::RawDocument;
use crate::state::EngineState;

pub async fn index(
    State(state): State<Arc<EngineState>>,
    Json(req): Json<IndexRequest>,
) -> Response {
    let now = crate::api::unix_now();
    let doc = RawDocument {
        url: req.url,
        title: req.title.unwrap_or_default(),
        body: req.body,
    };

    match state.ingest_document(doc, now) {
        Ok(doc_id) => (
            StatusCode::ACCEPTED,
            Json(IndexResponse { doc_id: doc_id.to_u64().to_string() }),
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
