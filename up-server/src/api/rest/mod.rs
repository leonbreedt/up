use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;

use crate::repository::RepositoryError;

pub mod check;
mod model;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("repository error: {0}")]
    Repository(#[from] RepositoryError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Repository(e) => match e {
                RepositoryError::InvalidArgument(_, _) => {
                    (StatusCode::BAD_REQUEST, format!("{}", e))
                }
                RepositoryError::NotFound => (StatusCode::NOT_FOUND, format!("{}", e)),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)),
            },
        };

        let body = Json(json!({
            "result": "failure",
            "message": message
        }));

        (status, body).into_response()
    }
}
