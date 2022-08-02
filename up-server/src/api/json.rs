use std::ops::{Deref, DerefMut};

use async_trait::async_trait;
use axum::body::{boxed, Bytes, Full};
use axum::extract::{FromRequest, RequestParts};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::BoxError;
use hyper::header::CONTENT_TYPE;
use miette::{Diagnostic, SourceOffset};
use mime_guess::mime;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use thiserror::Error;

use crate::api::{ReportRenderer, ReportType};
use crate::app::App;

/// Custom [`Json`] type to allow us to expose richer errors when deserialization
/// fails.
pub struct Json<T>(pub T);

impl<T> From<T> for Json<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}

impl<T> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        match serde_json::to_vec(&self.0) {
            Ok(bytes) => (
                [(
                    CONTENT_TYPE,
                    HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
                )],
                bytes,
            )
                .into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    CONTENT_TYPE,
                    HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
                )],
                err.to_string(),
            )
                .into_response(),
        }
    }
}

#[derive(Error, Debug, Diagnostic)]
#[error("{reason}")]
#[diagnostic(code(up::error::bad_request))]
struct JSONError<'s> {
    #[source_code]
    json: &'s str,
    line: usize,
    column: usize,
    reason: String,
    #[label("problem is here")]
    location: SourceOffset,
}

fn json_buf_response(status: StatusCode, buf: Vec<u8>) -> Response {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .body(boxed(Full::from(buf)))
        .expect("failed to build response")
}

fn json_response(status: StatusCode, body: Value) -> Response {
    json_buf_response(status, serde_json::to_vec(&body).unwrap())
}

fn print_error_report(err: &dyn Diagnostic) {
    if App::json_output() {
        println!("{}", ReportRenderer(ReportType::Json, err));
    } else {
        println!("Error: {}", ReportRenderer(ReportType::Graphical, err));
    }
}

#[async_trait]
impl<B, T> FromRequest<B> for Json<T>
where
    T: DeserializeOwned,
    B: axum::body::HttpBody + Send,
    B::Data: Send,
    B::Error: Into<BoxError>,
{
    type Rejection = Response;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let body_bytes = match Bytes::from_request(req).await {
            Ok(bytes) => bytes,
            Err(e) => {
                return Err(json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({
                        "result": "failure",
                        "message": format!("failed to read request body: {}", e)
                    }),
                ))
            }
        };

        let body_str = match std::str::from_utf8(&body_bytes) {
            Ok(s) => s,
            Err(e) => {
                return Err(json_response(
                    StatusCode::BAD_REQUEST,
                    json!({
                        "result": "failure",
                        "message": format!("request body is not UTF-8: {}", e)
                    }),
                ))
            }
        };

        match serde_json::from_str(body_str) {
            Ok(value) => Ok(Self(value)),
            Err(err) => {
                if err.is_syntax() || err.is_data() {
                    let reason = if err.is_syntax() {
                        format!(
                            "failed to parse JSON at line {}, column {}",
                            err.line(),
                            err.column()
                        )
                    } else {
                        format!("JSON is invalid: {}", err)
                    };

                    let json_err = JSONError {
                        json: body_str,
                        line: err.line(),
                        column: err.column(),
                        reason,
                        location: SourceOffset::from_location(
                            body_str,
                            err.line(),
                            err.column() + 1,
                        ),
                    };

                    print_error_report(&json_err);

                    Err(json_buf_response(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        format!("{}", ReportRenderer(ReportType::Json, &json_err))
                            .as_bytes()
                            .to_vec(),
                    ))
                } else {
                    Err(json_response(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        json!({
                        "result": "failure",
                        "message": format!("JSON parsing error: {}", err)
                        }),
                    ))
                }
            }
        }
    }
}
