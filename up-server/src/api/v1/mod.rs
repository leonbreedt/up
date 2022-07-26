use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
    Extension, Router,
};
use miette::Diagnostic;
use thiserror::Error;

use crate::{api::Json, app::App, auth::Identity, repository::RepositoryError};

use super::{GenericResponse, ReportRenderer, ReportType};

pub mod checks;
pub mod notifications;
pub mod ping;
pub mod projects;

#[derive(Error, Diagnostic, Debug)]
pub enum ApiError {
    #[error("repository error")]
    #[diagnostic(code(up::error::repository))]
    Repository(#[from] RepositoryError),
}

pub const PING_URI: &str = "/api/v1/ping";
pub const HEALTH_URI: &str = "/health";

pub fn router() -> Router {
    Router::new()
        .route("/api/v1/identity", get(identity_handler))
        // Projects
        .route("/api/v1/projects/:id", get(projects::read_one))
        .route("/api/v1/projects", get(projects::read_all))
        .route("/api/v1/projects", post(projects::create))
        .route("/api/v1/projects/:id", patch(projects::update))
        .route("/api/v1/projects/:id", delete(projects::delete))
        // Checks
        .route("/api/v1/projects/:id/checks/:id", get(checks::read_one))
        .route("/api/v1/projects/:id/checks", get(checks::read_all))
        .route("/api/v1/projects/:id/checks", post(checks::create))
        .route("/api/v1/projects/:id/checks/:id", patch(checks::update))
        .route("/api/v1/projects/:id/checks/:id", delete(checks::delete))
        // Notifications
        .route(
            "/api/v1/projects/:id/checks/:id/notifications/:id",
            get(notifications::read_one),
        )
        .route(
            "/api/v1/projects/:id/checks/:id/notifications",
            get(notifications::read_all),
        )
        .route(
            "/api/v1/projects/:id/checks/:id/notifications",
            post(notifications::create),
        )
        .route(
            "/api/v1/projects/:id/checks/:id/notifications/:id",
            patch(notifications::update),
        )
        .route(
            "/api/v1/projects/:id/checks/:id/notifications/:id",
            delete(notifications::delete),
        )
        // Miscellaneous
        .route(HEALTH_URI, get(health_handler))
        .route(&format!("{}/:key", PING_URI), post(ping::ping))
}

async fn health_handler() -> &'static str {
    "UP"
}

async fn identity_handler(Extension(identity): Extension<Identity>) -> impl IntoResponse {
    Json(identity)
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut details: Vec<String> = Vec::new();

        let (status, message) = match self {
            ApiError::Repository(e) => {
                if e.is_unique_constraint_violation() {
                    (
                        StatusCode::CONFLICT,
                        "already exists with name/key".to_string(),
                    )
                } else {
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
                        RepositoryError::Forbidden => (StatusCode::FORBIDDEN, format!("{}", e)),
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
            }
        };

        let body = if details.is_empty() {
            Json(GenericResponse::failure(message))
        } else {
            Json(GenericResponse::failure_with_details(message, details))
        };

        (status, body).into_response()
    }
}
