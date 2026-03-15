use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use tracing::info;

use crate::{
    agents::{AgentTask, TaskType},
    models::responses::{ErrorResponse, IntelligenceResponse},
    AppState,
};

pub async fn handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<IntelligenceResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("GET /api/intelligence");

    // Check cache
    let cache_key = "intelligence:latest";
    if let Some(cached) = state.cache.get::<IntelligenceResponse>(cache_key) {
        return Ok(Json(cached));
    }

    // Get from database
    let events = state.db.get_recent_events(100, 24).await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch intelligence")
        })?;

    let response = IntelligenceResponse {
        total: events.len(),
        updated_at: chrono::Utc::now().timestamp_millis(),
        events,
    };

    // Cache for 5 minutes
    state.cache.set(cache_key, &response, 300);

    Ok(Json(response))
}
