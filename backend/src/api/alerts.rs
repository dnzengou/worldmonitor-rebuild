use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    Json as AxumJson,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    models::responses::{ErrorResponse, SuccessResponse},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateAlertRequest {
    pub user_id: String,
    pub country: String,
    pub threshold: i32,
}

pub async fn create_handler(
    State(state): State<Arc<AppState>>,
    AxumJson(request): AxumJson<CreateAlertRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Get user to check tier limits
    let user = state.db.get_or_create_user(&request.user_id).await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get user")
        })?;

    let current_alerts = state.db.count_alerts(&request.user_id).await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to count alerts")
        })?;

    if current_alerts >= user.max_alerts() as i64 {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: format!(
                    "Alert limit reached ({} max). Upgrade to Pro for more.",
                    user.max_alerts()
                ),
                code: Some("alert_limit_reached".to_string()),
            }),
        ));
    }

    state.db.create_alert(&request.user_id, &request.country, request.threshold).await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create alert")
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        message: Some("Alert created successfully".to_string()),
    }))
}

pub async fn list_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // This would get alerts for the authenticated user
    Ok(Json(serde_json::json!({ "alerts": [] })))
}
