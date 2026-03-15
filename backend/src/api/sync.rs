use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    models::responses::{ErrorResponse},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct SyncParams {
    pub since: i64,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SyncParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let events = state.db.get_events_since(params.since).await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to sync")
        })?;

    Ok(Json(serde_json::json!({
        "new_events": events,
        "server_time": chrono::Utc::now().timestamp_millis(),
    })))
}
