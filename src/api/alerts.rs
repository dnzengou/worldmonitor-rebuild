use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    Json as AxumJson,
};
use std::sync::Arc;
use tracing::{info, warn};

use crate::{
    models::{requests::AlertRequest, responses::ErrorResponse, SuccessResponse},
    AppState,
};

/// POST /api/alerts - Subscribe to alerts for a country
pub async fn handler(
    State(state): State<Arc<AppState>>,
    AxumJson(request): AxumJson<AlertRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Handling POST /api/alerts for user: {}, country: {}",
        request.user_id, request.country
    );

    // Check free tier limit
    match state.db.count_alerts(&request.user_id).await {
        Ok(count) => {
            if count >= state.config.max_alerts_free as i64 {
                warn!(
                    "User {} exceeded free tier alert limit ({})",
                    request.user_id, state.config.max_alerts_free
                );
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse {
                        error: format!(
                            "Free tier limit reached ({} alerts max). Upgrade to Pro for unlimited alerts.",
                            state.config.max_alerts_free
                        ),
                    }),
                ));
            }
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to check alert limit".to_string(),
                }),
            ));
        }
    }

    // Create alert
    match state
        .db
        .create_alert(&request.user_id, &request.country, request.threshold)
        .await
    {
        Ok(_) => {
            info!(
                "Alert created for user: {}, country: {}",
                request.user_id, request.country
            );
            Ok(Json(SuccessResponse { success: true }))
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create alert".to_string(),
                }),
            ));
        }
    }
}
