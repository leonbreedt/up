use axum::body::Empty;
use axum::response::IntoResponse;
use axum::{extract::Path, Extension, Json};

use crate::api::rest::{model::check, ApiError};
use crate::repository::{dto, Repository};

pub async fn read_one(
    Path(id): Path<String>,
    repository: Extension<Repository>,
) -> Result<Json<check::Check>, ApiError> {
    let check: check::Check = repository
        .read_one_check(dto::check::Field::all(), &id)
        .await?
        .into();
    Ok(check.into())
}

pub async fn read_all(
    Extension(repository): Extension<Repository>,
) -> Result<Json<Vec<check::Check>>, ApiError> {
    let checks: Vec<check::Check> = repository
        .read_checks(dto::check::Field::all())
        .await?
        .into_iter()
        .map(|i| i.into())
        .collect();
    Ok(checks.into())
}

pub async fn create(
    repository: Extension<Repository>,
    request: Json<check::Create>,
) -> Result<Json<check::Check>, ApiError> {
    let check = repository
        .create_check(
            dto::check::Field::all(),
            &request.account_id.to_string(),
            &request.name,
        )
        .await?;
    let check: check::Check = check.into();
    Ok(check.into())
}

pub async fn update(
    Path(id): Path<String>,
    repository: Extension<Repository>,
    request: Json<check::Update>,
) -> Result<Json<check::Check>, ApiError> {
    let mut update_fields = Vec::new();
    if let Some(name) = &request.name {
        update_fields.push((dto::check::Field::Name, name.as_str().into()));
    }
    let (_, check) = repository
        .update_check(&id, dto::check::Field::all(), update_fields)
        .await?;
    let check: check::Check = check.into();
    Ok(check.into())
}

pub async fn delete(
    Path(id): Path<String>,
    Extension(repository): Extension<Repository>,
) -> Result<impl IntoResponse, ApiError> {
    repository.delete_check(&id).await?;
    Ok(Empty::new())
}
