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

/// Handler for `GET /api/v1/projects/:id/checks/:id`
pub async fn read_one(
    Path((project_id, check_id)): Path<(ShortId, ShortId)>,
    Extension(identity): Extension<Identity>,
    repository: Extension<Repository>,
) -> Result<Json<Check>, ApiError> {
    let check: Check = repository
        .check()
        .read_one(&identity, project_id.as_uuid(), check_id.as_uuid())
        .await?
        .into();
    Ok(check.into())
}

/// Handler for `GET /api/v1/projects/:id/checks`
pub async fn read_all(
    Path(project_id): Path<ShortId>,
    Extension(identity): Extension<Identity>,
    Extension(repository): Extension<Repository>,
) -> Result<Json<Vec<Check>>, ApiError> {
    let checks: Vec<Check> = repository
        .check()
        .read_all(&identity, project_id.as_uuid())
        .await?
        .into_iter()
        .map(|i| i.into())
        .collect();
    Ok(checks.into())
}

/// Handler for `POST /api/v1/projects/:id/checks`
pub async fn create(
    Path(project_id): Path<ShortId>,
    Extension(identity): Extension<Identity>,
    Extension(repository): Extension<Repository>,
    request: Json<CreateCheck>,
) -> Result<Json<Check>, ApiError> {
    let check: Check = repository
        .check()
        .create(&identity, project_id.as_uuid(), request.0.into())
        .await?
        .into();
    Ok(check.into())
}

/// Handler for `PATCH /api/v1/projects/:id/checks/:id`
pub async fn update(
    Path((project_id, check_id)): Path<(ShortId, ShortId)>,
    Extension(identity): Extension<Identity>,
    Extension(repository): Extension<Repository>,
    request: Json<UpdateCheck>,
) -> Result<Json<Check>, ApiError> {
    let check: Check = repository
        .check()
        .update(
            &identity,
            project_id.as_uuid(),
            check_id.as_uuid(),
            request.0.into(),
        )
        .await?
        .into();
    Ok(check.into())
}

/// Handler for `DELETE /api/v1/projects/:id/checks/:id`
pub async fn delete(
    Path((project_id, check_id)): Path<(ShortId, ShortId)>,
    Extension(identity): Extension<Identity>,
    Extension(repository): Extension<Repository>,
) -> Result<impl IntoResponse, ApiError> {
    repository
        .check()
        .delete(&identity, project_id.as_uuid(), check_id.as_uuid())
        .await?;
    Ok(Empty::new())
}

// API model types

/// An API [`Check`] type.
#[derive(Debug, Serialize, Deserialize)]
pub struct Check {
    pub id: ShortId,
    pub name: String,
    pub description: String,
    pub status: CheckStatus,
    pub schedule_type: ScheduleType,
    pub ping_period: i32,
    pub ping_period_units: PeriodUnits,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ping_cron_expression: Option<String>,
    pub grace_period: i32,
    pub grace_period_units: PeriodUnits,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_ping_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// An API check status.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CheckStatus {
    Up,
    Down,
    Created,
}

/// An API check schedule type.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScheduleType {
    Simple,
    Cron,
}

/// An API check period units type.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PeriodUnits {
    Minutes,
    Hours,
    Days,
}

/// Body for `POST /api/v1/projects/:id/checks`
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCheck {
    // TODO: remove, this should be part of logged in context
    pub account_id: ShortId,
    pub project_id: ShortId,
    pub name: String,
}

/// Body for `PATCH /api/v1/projects/:id/checks`
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCheck {
    pub name: Option<String>,
}

// Model conversions

/// Conversion from repository [`dto::Check`] to
/// API [`Check`].
impl From<dto::Check> for Check {
    fn from(issue: dto::Check) -> Self {
        Self {
            id: issue.uuid.into(),
            name: issue.name,
            description: issue.description,
            status: issue.status.into(),
            schedule_type: issue.schedule_type.into(),
            ping_period: issue.ping_period,
            ping_period_units: issue.ping_period_units.into(),
            ping_cron_expression: issue.ping_cron_expression,
            grace_period: issue.grace_period,
            grace_period_units: issue.grace_period_units.into(),
            last_ping_at: issue.last_ping_at.map(|d| Utc.from_utc_datetime(&d)),
            created_at: Utc.from_utc_datetime(&issue.created_at),
            updated_at: issue.updated_at.map(|d| Utc.from_utc_datetime(&d)),
        }
    }
}

/// Conversion from repository [`dto::CheckStatus`] to
/// API [`CheckStatus`].
impl From<dto::CheckStatus> for CheckStatus {
    fn from(status: dto::CheckStatus) -> Self {
        match status {
            dto::CheckStatus::Up => CheckStatus::Up,
            dto::CheckStatus::Down => CheckStatus::Down,
            dto::CheckStatus::Created => CheckStatus::Created,
        }
    }
}

/// Conversion from repository [`dto::ScheduleType`] to
/// API [`ScheduleType`].
impl From<dto::ScheduleType> for ScheduleType {
    fn from(status: dto::ScheduleType) -> Self {
        match status {
            dto::ScheduleType::Simple => ScheduleType::Simple,
            dto::ScheduleType::Cron => ScheduleType::Cron,
        }
    }
}

/// Conversion from repository [`dto::PeriodUnits`] to
/// API [`PeriodUnits`].
impl From<dto::PeriodUnits> for PeriodUnits {
    fn from(status: dto::PeriodUnits) -> Self {
        match status {
            dto::PeriodUnits::Minutes => PeriodUnits::Minutes,
            dto::PeriodUnits::Hours => PeriodUnits::Hours,
            dto::PeriodUnits::Days => PeriodUnits::Days,
        }
    }
}

/// Conversion from API [`CreateCheck`] to
/// repository [`dto::CreateCheck`].
impl From<CreateCheck> for dto::CreateCheck {
    fn from(request: CreateCheck) -> Self {
        Self {
            account_uuid: request.account_id.into_uuid(),
            project_uuid: request.project_id.into_uuid(),
            name: request.name,
        }
    }
}

/// Conversion from API [`UpdateCheck`] to
/// repository [`dto::UpdateCheck`].
impl From<UpdateCheck> for dto::UpdateCheck {
    fn from(request: UpdateCheck) -> Self {
        Self { name: request.name }
    }
}
