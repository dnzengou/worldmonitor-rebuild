use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    Json as AxumJson,
};
use chrono::Utc;
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    models::responses::{ErrorResponse, UserResponse},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub interests: Vec<String>,
    pub countries: Vec<String>,
}

pub async fn get_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<UserResponse>, (StatusCode, Json<ErrorResponse>)> {
    // In production, get user_id from JWT
    let user_id = "anonymous";

    let user = state.db.get_or_create_user(user_id).await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get user")
        })?;

    // Calculate streak
    let now = Utc::now();
    let last_visit = user.last_visit.unwrap_or(now);
    let diff_days = (now - last_visit).num_days();

    let new_streak = if diff_days == 1 {
        user.streak + 1
    } else if diff_days > 1 {
        0
    } else {
        user.streak
    };

    if diff_days >= 1 {
        let _ = state.db.update_streak(user_id, new_streak).await;
    }

    let is_new = user.last_visit.is_none();

    Ok(Json(UserResponse {
        id: user.id,
        tier: user.tier,
        streak: new_streak,
        interests: user.get_interests(),
        countries: user.get_countries(),
        max_alerts: user.max_alerts(),
        data_delay_hours: user.data_delay_hours(),
        is_new,
    }))
}

pub async fn update_handler(
    State(state): State<Arc<AppState>>,
    AxumJson(request): AxumJson<UpdateUserRequest>,
) -> Result<Json<UserResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = "anonymous";

    let mut user = state.db.get_or_create_user(user_id).await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get user")
        })?;

    user.interests = serde_json::to_string(&request.interests).unwrap_or_default();
    user.countries = serde_json::to_string(&request.countries).unwrap_or_default();

    state.db.update_user(&user).await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to update user")
        })?;

    Ok(Json(UserResponse {
        id: user.id,
        tier: user.tier,
        streak: user.streak,
        interests: request.interests,
        countries: request.countries,
        max_alerts: user.max_alerts(),
        data_delay_hours: user.data_delay_hours(),
        is_new: false,
    }))
}
