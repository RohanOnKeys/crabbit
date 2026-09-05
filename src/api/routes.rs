use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;

use crate::api::handlers::{health, index, search, stats};
use crate::state::EngineState;

pub fn build_router(state: Arc<EngineState>) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/search", get(search::search))
        .route("/index", post(index::index))
        .route("/stats", get(stats::stats))
        .with_state(state)
}
