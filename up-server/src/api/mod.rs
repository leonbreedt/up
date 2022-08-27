use std::sync::Arc;

use axum::{
    body::{boxed, Bytes},
    handler::Handler,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Extension, Router,
};
use hyper::{
    header::{CONTENT_LENGTH, CONTENT_TYPE},
    Body, Uri,
};
use miette::{Diagnostic, GraphicalReportHandler, JSONReportHandler, NarratableReportHandler};
use serde::{Deserialize, Serialize};
use up_core::jwt::Verifier;

mod json;
mod ui;
pub mod v1;

use crate::{api::json::Json, auth, notifier::Notifier, repository::Repository};

// Basic response status.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResponseStatus {
    Success,
    Failure,
}

/// Generic response for errors and/or simple APIs.
#[derive(Serialize, Deserialize, Debug)]
pub struct GenericResponse {
    pub status: ResponseStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    pub details: Vec<String>,
}

impl GenericResponse {
    fn success<S: AsRef<str>>(message: S) -> Self {
        Self {
            status: ResponseStatus::Success,
            message: Some(message.as_ref().to_string()),
            details: Vec::new(),
        }
    }

    fn failure<S: AsRef<str>>(message: S) -> Self {
        Self {
            status: ResponseStatus::Failure,
            message: Some(message.as_ref().to_string()),
            details: Vec::new(),
        }
    }

    fn failure_with_details<S: AsRef<str>>(message: S, details: Vec<String>) -> Self {
        Self {
            status: ResponseStatus::Failure,
            message: Some(message.as_ref().to_string()),
            details,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NotFoundResponse {
    #[serde(flatten)]
    inner: GenericResponse,
    path: String,
}

impl NotFoundResponse {
    pub fn new(uri: Uri) -> Self {
        Self {
            inner: GenericResponse::failure("not found"),
            path: uri.path().to_string(),
        }
    }
}

/// Builds a new router, providing handlers with a [`Repository`]
/// connected to the specified [`Database`].
pub fn build(repository: Repository, notifier: Notifier, verifier: Arc<Verifier>) -> Router {
    let router = v1::router()
        .route("/", get(ui::index_handler))
        .layer(Extension(notifier))
        .layer(middleware::from_fn(error_middleware))
        .layer(middleware::from_fn(auth::auth_middleware))
        .layer(Extension(repository))
        .layer(Extension(verifier))
        .fallback(not_found_handler.into_service());

    ui::Asset::register_routes(router)
}

/// Fallback handler for non-matching routes.
async fn not_found_handler(uri: Uri) -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Json(NotFoundResponse::new(uri)))
}

/// Error handling middleware that converts error responses (e.g. from extractors)
/// into JSON responses if required. Ideally we should implement handling of all
/// extractor rejections, but for now, we do it via a middleware.
async fn error_middleware<B>(req: Request<B>, next: Next<B>) -> Response {
    let response = next.run(req).await;
    let (mut head, body) = response.into_parts();
    let body_bytes = hyper::body::to_bytes(body)
        .await
        .expect("failed to convert error response into bytes");
    let body_bytes_len = body_bytes.len();

    let (body, size) = if !head.status.is_success() {
        if let Some(value) = head.headers.get(CONTENT_TYPE) {
            if value != "application/json" {
                let json_body = serde_json::to_string(&GenericResponse::failure(
                    std::str::from_utf8(&body_bytes).expect("failed to parse error response"),
                ))
                .expect("failed to create error JSON body");

                let bytes = Bytes::from(json_body.as_bytes().to_vec());
                let size = bytes.len();

                head.headers
                    .insert(CONTENT_TYPE, "application/json".parse().unwrap());

                (Body::from(bytes), size)
            } else {
                (Body::from(body_bytes), body_bytes_len)
            }
        } else {
            (Body::from(body_bytes), body_bytes_len)
        }
    } else {
        let size = body_bytes.len();
        (Body::from(body_bytes), size)
    };

    head.headers.insert(CONTENT_LENGTH, size.into());

    Response::from_parts(head, boxed(body))
}

pub(crate) enum ReportType {
    Json,
    Graphical,
    Narratable,
}

/// Helper for easily rendering [`Diagnostic`] into different output formats.
pub(crate) struct ReportRenderer<'e>(pub ReportType, pub &'e dyn Diagnostic);

impl<'e> std::fmt::Display for ReportRenderer<'e> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.0 {
            ReportType::Json => JSONReportHandler::new().render_report(f, self.1),
            ReportType::Graphical => GraphicalReportHandler::new().render_report(f, self.1),
            ReportType::Narratable => NarratableReportHandler::new().render_report(f, self.1),
        }
    }
}
