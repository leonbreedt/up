use axum::{body::Empty, extract::Path, response::IntoResponse, Extension};
use chrono::{DateTime, TimeZone, Utc};
use miette::Result;
use serde::{Deserialize, Serialize};

use crate::auth::Identity;
use crate::{
    api::{v1::ApiError, Json},
    repository::{dto, Repository},
    shortid::ShortId,
};

/// Handler for `GET /api/v1/projects/:id`
pub async fn read_one(
    Path(id): Path<ShortId>,
    Extension(identity): Extension<Identity>,
    repository: Extension<Repository>,
) -> Result<Json<Project>, ApiError> {
    let project: Project = repository
        .project()
        .read_one(&identity, id.as_uuid())
        .await?
        .into();
    Ok(project.into())
}

/// Handler for `GET /api/v1/projects`
pub async fn read_all(
    Extension(repository): Extension<Repository>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<Vec<Project>>, ApiError> {
    let projects: Vec<Project> = repository
        .project()
        .read_all(&identity)
        .await?
        .into_iter()
        .map(|i| i.into())
        .collect();
    Ok(projects.into())
}

/// Handler for `POST /api/v1/projects`
pub async fn create(
    Extension(repository): Extension<Repository>,
    Extension(identity): Extension<Identity>,
    request: Json<CreateProject>,
) -> Result<Json<Project>, ApiError> {
    let project: Project = repository
        .project()
        .create(&identity, request.0.into())
        .await?
        .into();
    Ok(project.into())
}

/// Handler for `PUT /api/v1/projects/:id`
pub async fn update(
    Path(id): Path<ShortId>,
    Extension(repository): Extension<Repository>,
    Extension(identity): Extension<Identity>,
    request: Json<UpdateProject>,
) -> Result<Json<Project>, ApiError> {
    let project: Project = repository
        .project()
        .update(&identity, id.as_uuid(), request.0.into())
        .await?
        .into();
    Ok(project.into())
}

/// Handler for `DELETE /api/v1/projects/:id`
pub async fn delete(
    Path(id): Path<ShortId>,
    Extension(repository): Extension<Repository>,
    Extension(identity): Extension<Identity>,
) -> Result<impl IntoResponse, ApiError> {
    repository.project().delete(&identity, id.as_uuid()).await?;
    Ok(Empty::new())
}

// API model types

/// An API [`Project`] type.
#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: ShortId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Body for `POST /api/v1/projects`
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateProject {
    pub account_id: ShortId,
    pub name: String,
}

/// Body for `PUT /api/v1/projects/:id`
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProject {
    pub name: Option<String>,
}

// Model conversions

/// Conversion from repository [`dto::Project`] to
/// API [`Project`].
impl From<dto::Project> for Project {
    fn from(issue: dto::Project) -> Self {
        Self {
            id: issue.uuid.into(),
            name: issue.name,
            created_at: Utc.from_utc_datetime(&issue.created_at),
            updated_at: issue.updated_at.map(|dt| Utc.from_utc_datetime(&dt)),
        }
    }
}

impl From<CreateProject> for dto::CreateProject {
    fn from(request: CreateProject) -> Self {
        Self {
            account_uuid: request.account_id.into_uuid(),
            name: request.name,
        }
    }
}

impl From<UpdateProject> for dto::UpdateProject {
    fn from(request: UpdateProject) -> Self {
        Self { name: request.name }
    }
}
