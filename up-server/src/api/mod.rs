use axum::{
    body::{boxed, Bytes},
    handler::Handler,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
    Extension, Json, Router,
};
use hyper::{Body, Uri};
use serde_json::json;

mod rest;
mod ui;

use crate::{database::Database, repository::Repository};

pub fn build(database: Database) -> Router {
    let repository = Repository::new(database);

    let router = Router::new()
        .route("/api/checks", get(rest::check::read_all))
        .route("/api/checks", post(rest::check::create))
        .route("/api/checks/:id", get(rest::check::read_one))
        .route("/api/checks/:id", patch(rest::check::update))
        .route("/api/checks/:id", delete(rest::check::delete))
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
    let (head, body) = response.into_parts();
    let body_bytes = hyper::body::to_bytes(body)
        .await
        .expect("failed to convert error response into bytes");

    let body = if head.status == StatusCode::UNPROCESSABLE_ENTITY {
        let json_body = serde_json::to_string(&json!({
            "result": "failure",
            "message": std::str::from_utf8(&body_bytes).expect("failed to parse error response"),
        }))
        .expect("failed to create error JSON body");

        Body::from(Bytes::from(json_body.as_bytes().to_vec()))
    } else {
        Body::from(body_bytes)
    };

    Response::from_parts(head, boxed(body))
}
