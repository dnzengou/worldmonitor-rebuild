use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use crate::{
    models::responses::{ErrorResponse},
    AppState,
};

pub async fn handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let events = state.db.get_recent_events(100, 24).await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch geo data")
        })?;

    let features: Vec<serde_json::Value> = events.iter().map(|e| {
        serde_json::json!({
            "type": "Feature",
            "geometry": {
                "type": "Point",
                "coordinates": [e.lon, e.lat],
            },
            "properties": {
                "country": e.country,
                "severity": e.severity,
                "headline": e.headline,
            },
        })
    }).collect();

    Ok(Json(serde_json::json!({
        "type": "FeatureCollection",
        "features": features,
    })))
}
