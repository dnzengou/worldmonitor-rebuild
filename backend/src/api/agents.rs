use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use crate::{
    models::responses::{AgentStatusResponse, ErrorResponse},
    AppState,
};

pub async fn status_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AgentStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    let agents = state.coordinator.health_check().await;
    let metrics = state.coordinator.get_metrics().await;

    Ok(Json(AgentStatusResponse { agents, metrics }))
}
