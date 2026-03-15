use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use crate::{
    cache::{strategies::*, Cache},
    core::{BriefGenerator, IntelligenceFusion},
    models::{
        requests::*,
        responses::*,
        GeoJson, IntelEvent, SyncResponse, User,
    },
    AppState,
};

// Re-export handlers
pub mod alerts;
pub mod brief;
pub mod geo;
pub mod intelligence;
pub mod sync;
pub mod user;

/// Helper function to extract user ID from Authorization header
pub fn extract_user_id(auth_header: Option<&str>) -> String {
    auth_header
        .and_then(|h| h.strip_prefix("Bearer "))
        .unwrap_or("anonymous")
        .to_string()
}

/// Helper to create error response
pub fn error_response(status: StatusCode, message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            error: message.to_string(),
        }),
    )
}
