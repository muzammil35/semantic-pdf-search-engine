use axum::{
    extract::{Query, State},
};
use axum::{
    response::{IntoResponse, Json, Response},
    http::StatusCode,
};
use std::collections::HashMap;
use serde_json::json;
use crate::types::{AppState};

// handlers/ready.rs
pub async fn is_ready(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Some(id) = params.get("id") {
        let ready = state.ready_set.read().await.contains(id);
        Json(json!({ "ready": ready })).into_response()
    } else {
        (StatusCode::BAD_REQUEST, "Missing id").into_response()
    }
}