use axum::{
    response::{IntoResponse, Response},
    extract::Json,
    http::StatusCode,
};
use serde_json::json;

pub enum AppError {
    NotFound(String),
    Internal(String),
    Unauthorized(String),
    BadRequest(String)
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
        };
        (status, Json(json!({"error": message}))).into_response()
    }
}


impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Internal(msg) => write!(f, "{msg}"),
            AppError::NotFound(msg) =>  write!(f, "{msg}"),
            AppError::Unauthorized(msg) => write!(f, "{msg}"),
            AppError::BadRequest(msg) => write!(f, "{msg}"),
        }
    }
}
