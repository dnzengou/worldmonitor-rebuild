//! API handlers

pub mod agents;
pub mod alerts;
pub mod brief;
pub mod geo;
pub mod intelligence;
pub mod payments;
pub mod sync;
pub mod user;

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use crate::AppState;

pub fn error_response(status: StatusCode, message: &str) -> (StatusCode, Json<crate::models::responses::ErrorResponse>) {
    (
        status,
        Json(crate::models::responses::ErrorResponse {
            error: message.to_string(),
            code: None,
        }),
    )
}
