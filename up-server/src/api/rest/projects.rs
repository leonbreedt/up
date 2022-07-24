use axum::body::Empty;
use axum::response::IntoResponse;
use axum::{extract::Path, Extension, Json};

use crate::api::rest::{model::projects, ApiError};
use crate::repository::{dto, Repository};

pub async fn read_one(
    Path(id): Path<String>,
    repository: Extension<Repository>,
) -> Result<Json<projects::Project>, ApiError> {
    let project: projects::Project = repository
        .read_one_project(dto::project::Field::all(), &id)
        .await?
        .into();
    Ok(project.into())
}

pub async fn read_all(
    Extension(repository): Extension<Repository>,
) -> Result<Json<Vec<projects::Project>>, ApiError> {
    let projects: Vec<projects::Project> = repository
        .read_projects(dto::project::Field::all())
        .await?
        .into_iter()
        .map(|i| i.into())
        .collect();
    Ok(projects.into())
}

pub async fn create(
    repository: Extension<Repository>,
    request: Json<projects::Create>,
) -> Result<Json<projects::Project>, ApiError> {
    let project = repository
        .create_project(
            dto::project::Field::all(),
            &request.account_id.to_string(),
            &request.name,
        )
        .await?;
    let project: projects::Project = project.into();
    Ok(project.into())
}

pub async fn update(
    Path(id): Path<String>,
    repository: Extension<Repository>,
    request: Json<projects::Update>,
) -> Result<Json<projects::Project>, ApiError> {
    let mut update_fields = Vec::new();
    if let Some(name) = &request.name {
        update_fields.push((dto::project::Field::Name, name.as_str().into()));
    }
    let (_, project) = repository
        .update_project(&id, dto::project::Field::all(), update_fields)
        .await?;
    let project: projects::Project = project.into();
    Ok(project.into())
}

pub async fn delete(
    Path(id): Path<String>,
    Extension(repository): Extension<Repository>,
) -> Result<impl IntoResponse, ApiError> {
    repository.delete_project(&id).await?;
    Ok(Empty::new())
}