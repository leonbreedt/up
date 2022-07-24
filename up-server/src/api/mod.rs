use axum::{
    routing::{delete, get, patch, post},
    Extension, Router,
};

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
        .layer(Extension(repository));

    ui::Asset::register_routes(router)
}
