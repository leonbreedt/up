use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, patch, post};
use axum::Router;
use miette::Diagnostic;
use serde_json::json;
use thiserror::Error;

use crate::api::Json;
use crate::app::App;
use crate::repository::RepositoryError;

use super::{ReportRenderer, ReportType};

pub mod checks;
pub mod projects;

#[derive(Error, Diagnostic, Debug)]
pub enum ApiError {
    #[error("repository error")]
    #[diagnostic(code(up::error::repository))]
    Repository(#[from] RepositoryError),
}

pub fn router() -> Router {
    Router::new()
        .route("/api/v1/checks", get(checks::read_all))
        .route("/api/v1/checks", post(checks::create))
        .route("/api/v1/checks/:id", get(checks::read_one))
        .route("/api/v1/checks/:id", patch(checks::update))
        .route("/api/v1/checks/:id", delete(checks::delete))
        .route("/api/v1/projects", get(projects::read_all))
        .route("/api/v1/projects", post(projects::create))
        .route("/api/v1/projects/:id", get(projects::read_one))
        .route("/api/v1/projects/:id", patch(projects::update))
        .route("/api/v1/projects/:id", delete(projects::delete))
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut details: Vec<String> = Vec::new();

        let (status, message) = match self {
            ApiError::Repository(e) => {
                if App::json_output() {
                    println!("{}", ReportRenderer(ReportType::Json, &e));
                } else {
                    println!("Error: {}", ReportRenderer(ReportType::Graphical, &e));
                }

                match e {
                    RepositoryError::NotFound { entity_type, id } => (
                        StatusCode::NOT_FOUND,
                        format!("{} with ID {} does not exist", entity_type, id),
                    ),
                    _ => {
                        let mut messages: Vec<String> =
                            format!("{}", ReportRenderer(ReportType::Narratable, &e))
                                .split('\n')
                                .map(|s| s.trim())
                                .filter(|s| !s.is_empty())
                                .map(|s| s.to_string())
                                .collect();

                        let message = messages.remove(0);
                        for detail in messages.into_iter() {
                            details.push(detail);
                        }

                        (StatusCode::INTERNAL_SERVER_ERROR, message)
                    }
                }
            }
        };

        let body = if details.is_empty() {
            Json(json!({
                "result": "failure",
                "message": message
            }))
        } else {
            Json(json!({
                "result": "failure",
                "message": message,
                "details": details
            }))
        };

        (status, body).into_response()
    }
}
