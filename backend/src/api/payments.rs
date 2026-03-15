use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    Json as AxumJson,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    models::{responses::ErrorResponse, UserTier},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct SubscribeRequest {
    pub tier: UserTier,
}

pub async fn pricing_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let tiers = vec![
        serde_json::json!({
            "id": "free",
            "name": "Free",
            "price": 0,
            "price_string": "Free",
            "features": state.payments.get_tier_features(UserTier::Free),
        }),
        serde_json::json!({
            "id": "pro",
            "name": "Pro",
            "price": 900,
            "price_string": "$9/month",
            "features": state.payments.get_tier_features(UserTier::Pro),
        }),
        serde_json::json!({
            "id": "enterprise",
            "name": "Enterprise",
            "price": 29900,
            "price_string": "$299/month",
            "features": state.payments.get_tier_features(UserTier::Enterprise),
        }),
    ];

    Json(serde_json::json!({ "tiers": tiers }))
}

pub async fn subscribe_handler(
    State(state): State<Arc<AppState>>,
    AxumJson(request): AxumJson<SubscribeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if !state.payments.is_configured() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "Payment processing not configured".to_string(),
                code: Some("payments_not_configured".to_string()),
            }),
        ));
    }

    // In a real implementation, this would create a Stripe checkout session
    let session_id = state.payments.create_checkout_session("user_id", request.tier).await
        .map_err(|e| {
            tracing::error!("Failed to create checkout session: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create checkout session")
        })?;

    Ok(Json(serde_json::json!({
        "checkout_url": format!("/checkout/{}", session_id),
    })))
}
