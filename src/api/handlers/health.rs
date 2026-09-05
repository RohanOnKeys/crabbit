use axum::Json;

use crate::api::dto::HealthResponse;

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
