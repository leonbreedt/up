use axum::{body::Empty, extract::Path, response::IntoResponse, Extension};
use chrono::{DateTime, TimeZone, Utc};
use miette::Result;
use serde::{Deserialize, Serialize};

use crate::{
    api::{v1::ApiError, Json},
    repository::{column_expression, column_value, dto, QueryValue, Repository},
    shortid::ShortId,
};

/// Handler for `GET /api/v1/checks/:id`
pub async fn read_one(
    Path(id): Path<ShortId>,
    repository: Extension<Repository>,
) -> Result<Json<Check>, ApiError> {
    let check: Check = repository.check().read_one(id.as_uuid()).await?.into();
    Ok(check.into())
}

/// Handler for `GET /api/v1/checks`
pub async fn read_all(
    Extension(repository): Extension<Repository>,
) -> Result<Json<Vec<Check>>, ApiError> {
    let checks: Vec<Check> = repository
        .check()
        .read_all()
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
        .check()
        .create(
            request.account_id.as_uuid(),
            request.project_id.as_uuid(),
            &request.name,
        )
        .await?;
    let check: Check = check.into();
    Ok(check.into())
}

/// Handler for `PATCH /api/v1/checks/:id`
pub async fn update(
    Path(id): Path<ShortId>,
    repository: Extension<Repository>,
    request: Json<UpdateCheck>,
) -> Result<Json<Check>, ApiError> {
    let mut update_fields = Vec::new();
    if let Some(name) = &request.name {
        update_fields.push((dto::CheckField::Name, name.as_str().into()));
    }
    let (_, check) = repository
        .check()
        .update(id.as_uuid(), update_fields)
        .await?;
    let check: Check = check.into();
    Ok(check.into())
}

/// Handler for `DELETE /api/v1/checks/:id`
pub async fn delete(
    Path(id): Path<ShortId>,
    Extension(repository): Extension<Repository>,
) -> Result<impl IntoResponse, ApiError> {
    repository.check().delete(id.as_uuid()).await?;
    Ok(Empty::new())
}

/// Handler for `GET /api/v1/checks/:id/notifications/:id`
pub async fn read_one_notification(
    Path((id, notification_id)): Path<(ShortId, ShortId)>,
    repository: Extension<Repository>,
) -> Result<Json<Notification>, ApiError> {
    let notification: Notification = repository
        .notification()
        .read_one(id.as_uuid(), notification_id.as_uuid())
        .await?
        .into();
    Ok(notification.into())
}

/// Handler for `GET /api/v1/checks/:id/notifications`
pub async fn read_all_notifications(
    Path(id): Path<ShortId>,
    Extension(repository): Extension<Repository>,
) -> Result<Json<Vec<Notification>>, ApiError> {
    let notifications: Vec<Notification> = repository
        .notification()
        .read_all(id.as_uuid())
        .await?
        .into_iter()
        .map(|i| i.into())
        .collect();
    Ok(notifications.into())
}

/// Handler for `POST /api/v1/checks/:id/notifications`
pub async fn create_notification(
    Path(id): Path<ShortId>,
    repository: Extension<Repository>,
    request: Json<CreateNotification>,
) -> Result<Json<Notification>, ApiError> {
    let notification = repository
        .notification()
        .create(id.as_uuid(), request.values())
        .await?;
    let notification: Notification = notification.into();
    Ok(notification.into())
}

/// Handler for `PATCH /api/v1/checks/:id/notifications/:id`
pub async fn update_notification(
    Path((id, notification_id)): Path<(ShortId, ShortId)>,
    repository: Extension<Repository>,
    request: Json<UpdateNotification>,
) -> Result<Json<Notification>, ApiError> {
    let (_, notification) = repository
        .notification()
        .update(id.as_uuid(), notification_id.as_uuid(), request.values())
        .await?;
    let notification: Notification = notification.into();
    Ok(notification.into())
}

/// Handler for `DELETE /api/v1/checks/:id/notifications/:id`
pub async fn delete_notification(
    Path((id, notification_id)): Path<(ShortId, ShortId)>,
    Extension(repository): Extension<Repository>,
) -> Result<impl IntoResponse, ApiError> {
    repository
        .notification()
        .delete(id.as_uuid(), notification_id.as_uuid())
        .await?;
    Ok(Empty::new())
}

// API model types

// Checks

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

/// Body for `POST /api/v1/checks`
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCheck {
    // TODO: remove, this should be part of logged in context
    pub account_id: ShortId,
    pub project_id: ShortId,
    pub name: String,
}

/// Body for `PATCH /api/v1/checks`
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCheck {
    pub name: Option<String>,
}

// Check model conversions

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

// Notifications

/// An API [`Notification`] type.
#[derive(Debug, Serialize, Deserialize)]
pub struct Notification {
    pub id: ShortId,
    pub name: String,
    #[serde(rename = "type")]
    pub notification_type: NotificationType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub max_retries: i32,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NotificationType {
    Email,
    Webhook,
}

/// Body for `POST /api/v1/notifications`.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateNotification {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub notification_type: NotificationType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<i32>,
}

impl CreateNotification {
    pub fn values(&self) -> Vec<QueryValue<dto::NotificationField>> {
        let mut values = Vec::new();
        if let Some(name) = self.name.as_deref() {
            values.push(column_value(dto::NotificationField::Name, name));
        }
        let notification_type: dto::NotificationType = (&self.notification_type).into();
        values.push(column_expression(
            dto::NotificationField::NotificationType,
            notification_type,
        ));
        if let Some(email) = self.email.as_deref() {
            values.push(column_value(dto::NotificationField::Email, email));
        }
        if let Some(url) = self.url.as_deref() {
            values.push(column_value(dto::NotificationField::Email, url));
        }
        if let Some(max_retries) = self.max_retries {
            values.push(column_value(
                dto::NotificationField::MaxRetries,
                max_retries,
            ));
        }
        values
    }
}

/// Body for `PUT /api/v1/notifications`.
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateNotification {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub notification_type: Option<NotificationType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<i32>,
}

impl UpdateNotification {
    pub fn values(&self) -> Vec<QueryValue<dto::NotificationField>> {
        let mut values = Vec::new();
        if let Some(name) = self.name.as_deref() {
            values.push(column_value(dto::NotificationField::Name, name));
        }
        if let Some(notification_type) = &self.notification_type {
            let notification_type: dto::NotificationType = notification_type.into();
            values.push(column_expression(
                dto::NotificationField::NotificationType,
                notification_type,
            ));
        }
        if let Some(email) = self.email.as_deref() {
            values.push(column_value(dto::NotificationField::Email, email));
        }
        if let Some(url) = self.url.as_deref() {
            values.push(column_value(dto::NotificationField::Email, url));
        }
        if let Some(max_retries) = self.max_retries {
            values.push(column_value(
                dto::NotificationField::MaxRetries,
                max_retries,
            ));
        }
        values
    }
}

// Notification model conversions

/// Conversion from repository [`dto::Notification`] to
/// API [`Notification`].
impl From<dto::Notification> for Notification {
    fn from(notification: dto::Notification) -> Self {
        Self {
            id: notification.uuid.into(),
            name: notification.name,
            notification_type: notification.notification_type.into(),
            email: notification.email,
            url: notification.url,
            max_retries: notification.max_retries,
            created_at: Utc.from_utc_datetime(&notification.created_at),
            updated_at: notification.updated_at.map(|d| Utc.from_utc_datetime(&d)),
        }
    }
}

/// Conversion from repository [`dto::NotificationType`] to
/// API [`NotificationType`].
impl From<dto::NotificationType> for NotificationType {
    fn from(notification_type: dto::NotificationType) -> Self {
        match notification_type {
            dto::NotificationType::Email => NotificationType::Email,
            dto::NotificationType::Webhook => NotificationType::Webhook,
        }
    }
}

impl From<&NotificationType> for dto::NotificationType {
    fn from(notification_type: &NotificationType) -> Self {
        match notification_type {
            NotificationType::Email => dto::NotificationType::Email,
            NotificationType::Webhook => dto::NotificationType::Webhook,
        }
    }
}
