use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub enum AppError {
    NotFound(String),
    Internal(String),
    Forbidden(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg)  => (StatusCode::NOT_FOUND, msg),
            AppError::Internal(msg)  => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
