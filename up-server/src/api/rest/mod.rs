use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use miette::{Diagnostic, GraphicalReportHandler, JSONReportHandler, NarratableReportHandler};
use serde_json::json;
use thiserror::Error;

use crate::app::App;
use crate::repository::RepositoryError;

pub mod checks;
pub mod projects;

mod model;

#[derive(Error, Diagnostic, Debug)]
pub enum ApiError {
    #[error("repository error")]
    #[diagnostic(code(up::error::repository))]
    Repository(#[from] RepositoryError),
}

enum ReportType {
    Json,
    Graphical,
    Narratable,
}

struct ReportRenderer<'e>(ReportType, &'e RepositoryError);

impl<'e> std::fmt::Display for ReportRenderer<'e> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.0 {
            ReportType::Json => JSONReportHandler::new().render_report(f, self.1),
            ReportType::Graphical => GraphicalReportHandler::new().render_report(f, self.1),
            ReportType::Narratable => NarratableReportHandler::new().render_report(f, self.1),
        }
    }
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
