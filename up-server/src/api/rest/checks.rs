use axum::body::Empty;
use axum::response::IntoResponse;
use axum::{extract::Path, Extension, Json};
use miette::Result;

use crate::api::rest::{model::checks, ApiError};
use crate::repository::{dto, Repository};
use crate::shortid::ShortId;

pub async fn read_one(
    Path(id): Path<ShortId>,
    repository: Extension<Repository>,
) -> Result<Json<checks::Check>, ApiError> {
    let check: checks::Check = repository
        .read_one_check(dto::check::Field::all(), id.as_uuid())
        .await?
        .into();
    Ok(check.into())
}

pub async fn read_all(
    Extension(repository): Extension<Repository>,
) -> Result<Json<Vec<checks::Check>>, ApiError> {
    let checks: Vec<checks::Check> = repository
        .read_checks(dto::check::Field::all())
        .await?
        .into_iter()
        .map(|i| i.into())
        .collect();
    Ok(checks.into())
}

pub async fn create(
    repository: Extension<Repository>,
    request: Json<checks::Create>,
) -> Result<Json<checks::Check>, ApiError> {
    let check = repository
        .create_check(
            dto::check::Field::all(),
            request.account_id.as_uuid(),
            request.project_id.as_uuid(),
            &request.name,
        )
        .await?;
    let check: checks::Check = check.into();
    Ok(check.into())
}

pub async fn update(
    Path(id): Path<ShortId>,
    repository: Extension<Repository>,
    request: Json<checks::Update>,
) -> Result<Json<checks::Check>, ApiError> {
    let mut update_fields = Vec::new();
    if let Some(name) = &request.name {
        update_fields.push((dto::check::Field::Name, name.as_str().into()));
    }
    let (_, check) = repository
        .update_check(id.as_uuid(), dto::check::Field::all(), update_fields)
        .await?;
    let check: checks::Check = check.into();
    Ok(check.into())
}

pub async fn delete(
    Path(id): Path<ShortId>,
    Extension(repository): Extension<Repository>,
) -> Result<impl IntoResponse, ApiError> {
    repository.delete_check(id.as_uuid()).await?;
    Ok(Empty::new())
}
