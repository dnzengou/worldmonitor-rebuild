use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    Json as AxumJson,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

use crate::{
    agents::{AgentTask, TaskType},
    models::responses::{BriefResponse, ErrorResponse},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct BriefRequest {
    pub country: String,
    #[serde(default)]
    pub include_reasoning: bool,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    AxumJson(request): AxumJson<BriefRequest>,
) -> Result<Json<BriefResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("POST /api/brief for country: {}", request.country);

    // Check cache
    let cache_key = format!("brief:{}", request.country);
    if let Some(cached) = state.cache.get::<BriefResponse>(&cache_key) {
        return Ok(Json(cached));
    }

    // Execute agent task
    let task = AgentTask::new(
        TaskType::GenerateBrief,
        &serde_json::json!({
            "country": request.country,
            "interests": [],
        }),
    )
    .map_err(|e| {
        tracing::error!("Failed to create task: {}", e);
        error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create task")
    })?;

    let result = state.coordinator.execute_task(task).await
        .map_err(|e| {
            tracing::error!("Agent execution failed: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate brief")
        })?;

    // Parse brief from result
    let brief: crate::agents::analyst::AnalysisBrief = serde_json::from_value(result.data)
        .map_err(|e| {
            tracing::error!("Failed to parse brief: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to parse brief")
        })?;

    let response = BriefResponse {
        country: brief.country,
        summary: brief.summary,
        event_count: brief.event_count,
        risk_level: brief.risk_level,
        generated_at: chrono::Utc::now().timestamp_millis(),
        chain_of_thought: if request.include_reasoning {
            Some(result.chain_of_thought)
        } else {
            None
        },
    };

    // Cache for 1 hour
    state.cache.set(&cache_key, &response, 3600);

    Ok(Json(response))
}
