use axum::body::Empty;
use axum::response::IntoResponse;
use axum::{extract::Path, Extension};
use chrono::{DateTime, Utc};
use miette::Result;
use serde::{Deserialize, Serialize};

use crate::api::v1::ApiError;
use crate::api::Json;
use crate::repository::{dto, Repository};
use crate::shortid::ShortId;

/// Handler for `GET /api/v1/checks/:id`
pub async fn read_one(
    Path(id): Path<ShortId>,
    repository: Extension<Repository>,
) -> Result<Json<Check>, ApiError> {
    let check: Check = repository
        .read_one_check(dto::check::Field::all(), id.as_uuid())
        .await?
        .into();
    Ok(check.into())
}

/// Handler for `GET /api/v1/checks`
pub async fn read_all(
    Extension(repository): Extension<Repository>,
) -> Result<Json<Vec<Check>>, ApiError> {
    let checks: Vec<Check> = repository
        .read_checks(dto::check::Field::all())
        .await?
        .into_iter()
        .map(|i| i.into())
        .collect();
    Ok(checks.into())
}

/// Handler for `POST /api/v1/checks`
pub async fn create(
    repository: Extension<Repository>,
    request: Json<CreateCheck>,
) -> Result<Json<Check>, ApiError> {
    let check = repository
        .create_check(
            dto::check::Field::all(),
            request.account_id.as_uuid(),
            request.project_id.as_uuid(),
            &request.name,
        )
        .await?;
    let check: Check = check.into();
    Ok(check.into())
}

/// Handler for `PUT /api/v1/checks/:id`
pub async fn update(
    Path(id): Path<ShortId>,
    repository: Extension<Repository>,
    request: Json<UpdateCheck>,
) -> Result<Json<Check>, ApiError> {
    let mut update_fields = Vec::new();
    if let Some(name) = &request.name {
        update_fields.push((dto::check::Field::Name, name.as_str().into()));
    }
    let (_, check) = repository
        .update_check(id.as_uuid(), dto::check::Field::all(), update_fields)
        .await?;
    let check: Check = check.into();
    Ok(check.into())
}

/// Handler for `DELETE /api/v1/checks/:id`
pub async fn delete(
    Path(id): Path<ShortId>,
    Extension(repository): Extension<Repository>,
) -> Result<impl IntoResponse, ApiError> {
    repository.delete_check(id.as_uuid()).await?;
    Ok(Empty::new())
}

/// Body for `POST /api/v1/checks`
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCheck {
    // TODO: remove, this should be part of logged in context
    pub account_id: ShortId,
    pub project_id: ShortId,
    pub name: String,
}

/// Body for `PUT /api/v1/checks`
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCheck {
    pub name: Option<String>,
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

/// An API [`Check`] type.
#[derive(Debug, Serialize, Deserialize)]
pub struct Check {
    pub id: ShortId,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: CheckStatus,
    pub schedule_type: ScheduleType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ping_period: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ping_period_units: Option<PeriodUnits>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ping_cron_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period_units: Option<PeriodUnits>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_ping_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Conversion from repository [`dto::check::Check`] to
/// API [`Check`].
impl From<dto::check::Check> for Check {
    fn from(issue: dto::check::Check) -> Self {
        Self {
            id: issue.uuid.unwrap().into(),
            name: issue.name.unwrap(),
            description: issue.description,
            status: issue.status.unwrap().into(),
            schedule_type: issue.schedule_type.unwrap().into(),
            ping_period: issue.ping_period,
            ping_period_units: issue.ping_period_units.map(|u| u.into()),
            ping_cron_expression: issue.ping_cron_expression,
            grace_period: issue.grace_period,
            grace_period_units: issue.grace_period_units.map(|u| u.into()),
            last_ping_at: issue.last_ping_at,
            created_at: issue.created_at.unwrap(),
            updated_at: issue.updated_at,
        }
    }
}

/// Conversion from repository [`dto::check::CheckStatus`] to
/// API [`CheckStatus`].
impl From<dto::check::CheckStatus> for CheckStatus {
    fn from(status: dto::check::CheckStatus) -> Self {
        match status {
            dto::check::CheckStatus::Up => CheckStatus::Up,
            dto::check::CheckStatus::Down => CheckStatus::Down,
            dto::check::CheckStatus::Created => CheckStatus::Created,
        }
    }
}

/// Conversion from repository [`dto::check::ScheduleType`] to
/// API [`ScheduleType`].
impl From<dto::check::ScheduleType> for ScheduleType {
    fn from(status: dto::check::ScheduleType) -> Self {
        match status {
            dto::check::ScheduleType::Simple => ScheduleType::Simple,
            dto::check::ScheduleType::Cron => ScheduleType::Cron,
        }
    }
}

/// Conversion from repository [`dto::check::PeriodUnits`] to
/// API [`PeriodUnits`].
impl From<dto::check::PeriodUnits> for PeriodUnits {
    fn from(status: dto::check::PeriodUnits) -> Self {
        match status {
            dto::check::PeriodUnits::Minutes => PeriodUnits::Minutes,
            dto::check::PeriodUnits::Hours => PeriodUnits::Hours,
            dto::check::PeriodUnits::Days => PeriodUnits::Days,
        }
    }
}
