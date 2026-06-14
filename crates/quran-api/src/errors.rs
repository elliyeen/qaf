use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ApiError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl PartialEq for ApiError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ApiError::NotFound(a), ApiError::NotFound(b)) => a == b,
            (ApiError::BadRequest(a), ApiError::BadRequest(b)) => a == b,
            (ApiError::Internal(a), ApiError::Internal(b)) => a.to_string() == b.to_string(),
            _ => false,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::Internal(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("internal error: {}", err),
            ),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_eq_same_message() {
        assert_eq!(
            ApiError::NotFound("ayah 1:1".into()),
            ApiError::NotFound("ayah 1:1".into()),
        );
    }

    #[test]
    fn not_found_ne_different_message() {
        assert_ne!(
            ApiError::NotFound("ayah 1:1".into()),
            ApiError::NotFound("ayah 1:2".into()),
        );
    }

    #[test]
    fn bad_request_eq_same_message() {
        assert_eq!(
            ApiError::BadRequest("invalid surah".into()),
            ApiError::BadRequest("invalid surah".into()),
        );
    }

    #[test]
    fn different_variants_are_not_equal() {
        assert_ne!(
            ApiError::NotFound("x".into()),
            ApiError::BadRequest("x".into()),
        );
    }

    #[test]
    fn internal_eq_same_message() {
        let a = ApiError::Internal(anyhow::anyhow!("db exploded"));
        let b = ApiError::Internal(anyhow::anyhow!("db exploded"));
        assert_eq!(a, b);
    }

    #[test]
    fn internal_ne_different_message() {
        let a = ApiError::Internal(anyhow::anyhow!("db exploded"));
        let b = ApiError::Internal(anyhow::anyhow!("connection timeout"));
        assert_ne!(a, b);
    }
}
