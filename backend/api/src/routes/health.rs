use axum::Json;
use serde_json::{Value, json};

#[utoipa::path(get, path = "/api/v1/health", tag = "runtime", responses((status = 200, body = Value)))]
pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "service": "fleet-control" }))
}
