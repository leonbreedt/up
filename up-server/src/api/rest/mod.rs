use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use miette::{Diagnostic, GraphicalReportHandler, JSONReportHandler};
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

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Repository(e) => {
                if App::json_output() {
                    let mut json = String::new();
                    JSONReportHandler::new()
                        .render_report(&mut json, &e)
                        .unwrap();
                    println!("{}", json);
                } else {
                    let mut out = String::new();
                    GraphicalReportHandler::new()
                        .render_report(&mut out, &e)
                        .unwrap();
                    println!("Error: {}", out);
                }

                match e {
                    RepositoryError::NotFound { entity_type, id } => {
                        (StatusCode::NOT_FOUND, format!("{} with ID {} does not exist", entity_type, id))
                    },
                    _ => (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)),
                }
            }
        };

        let body = Json(json!({
            "result": "failure",
            "message": message
        }));

        (status, body).into_response()
    }
}
