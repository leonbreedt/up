use axum::{
    body::{boxed, Bytes},
    handler::Handler,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
    Extension, Json, Router,
};
use hyper::{
    header::{CONTENT_LENGTH, CONTENT_TYPE},
    Body, Uri,
};
use serde_json::json;

mod rest;
mod ui;

use crate::{database::Database, repository::Repository};

pub fn build(database: Database) -> Router {
    let repository = Repository::new(database);

    let router = Router::new()
        .route("/api/checks", get(rest::checks::read_all))
        .route("/api/checks", post(rest::checks::create))
        .route("/api/checks/:id", get(rest::checks::read_one))
        .route("/api/checks/:id", patch(rest::checks::update))
        .route("/api/checks/:id", delete(rest::checks::delete))
        .route("/api/projects", get(rest::projects::read_all))
        .route("/api/projects", post(rest::projects::create))
        .route("/api/projects/:id", get(rest::projects::read_one))
        .route("/api/projects/:id", patch(rest::projects::update))
        .route("/api/projects/:id", delete(rest::projects::delete))
        .route("/", get(ui::index_handler))
        .layer(Extension(repository))
        .layer(middleware::from_fn(error_middleware))
        .fallback(not_found_handler.into_service());

    ui::Asset::register_routes(router)
}

async fn not_found_handler(uri: Uri) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "result": "failure",
            "message": "not found",
            "uri": uri.to_string()
        })),
    )
}

async fn error_middleware<B>(req: Request<B>, next: Next<B>) -> Response {
    let response = next.run(req).await;
    let (mut head, body) = response.into_parts();
    let body_bytes = hyper::body::to_bytes(body)
        .await
        .expect("failed to convert error response into bytes");

    let (body, size) = if head.status == StatusCode::UNPROCESSABLE_ENTITY {
        let json_body = serde_json::to_string(&json!({
            "result": "failure",
            "message": std::str::from_utf8(&body_bytes).expect("failed to parse error response"),
        }))
        .expect("failed to create error JSON body");

        let bytes = Bytes::from(json_body.as_bytes().to_vec());
        let size = bytes.len();

        head.headers
            .insert(CONTENT_TYPE, "application/json".parse().unwrap());

        (Body::from(bytes), size)
    } else {
        let size = body_bytes.len();
        (Body::from(body_bytes), size)
    };

    head.headers.insert(CONTENT_LENGTH, size.into());

    Response::from_parts(head, boxed(body))
}
