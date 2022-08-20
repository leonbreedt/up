use axum::{body::Empty, extract::Path, response::IntoResponse, Extension};
use chrono::{DateTime, TimeZone, Utc};
use miette::Result;
use serde::{Deserialize, Serialize};

use crate::{
    api::{v1::ApiError, Json},
    auth::Identity,
    repository::{dto, Repository},
    shortid::ShortId,
};

/// Handler for `GET /api/v1/projects/:id/checks/:id/notifications/:id`
pub async fn read_one(
    Path((project_id, check_id, notification_id)): Path<(ShortId, ShortId, ShortId)>,
    Extension(identity): Extension<Identity>,
    Extension(repository): Extension<Repository>,
) -> Result<Json<Notification>, ApiError> {
    let notification: Notification = repository
        .notification()
        .read_one(
            &identity,
            project_id.as_uuid(),
            check_id.as_uuid(),
            notification_id.as_uuid(),
        )
        .await?
        .into();
    Ok(notification.into())
}

/// Handler for `GET /api/v1/projects/:id/checks/:id/notifications`
pub async fn read_all(
    Path((project_id, check_id)): Path<(ShortId, ShortId)>,
    Extension(identity): Extension<Identity>,
    Extension(repository): Extension<Repository>,
) -> Result<Json<Vec<Notification>>, ApiError> {
    let notifications: Vec<Notification> = repository
        .notification()
        .read_all(&identity, project_id.as_uuid(), check_id.as_uuid())
        .await?
        .into_iter()
        .map(|i| i.into())
        .collect();
    Ok(notifications.into())
}

/// Handler for `POST /api/v1/projects/:id/checks/:id/notifications`
pub async fn create(
    Path((project_id, check_id)): Path<(ShortId, ShortId)>,
    Extension(identity): Extension<Identity>,
    Extension(repository): Extension<Repository>,
    request: Json<CreateNotification>,
) -> Result<Json<Notification>, ApiError> {
    let notification: Notification = repository
        .notification()
        .create(
            &identity,
            project_id.as_uuid(),
            check_id.as_uuid(),
            request.0.into(),
        )
        .await?
        .into();
    Ok(notification.into())
}

/// Handler for `PATCH /api/v1/projects/:id/checks/:id/notifications/:id`
pub async fn update(
    Path((project_id, check_id, notification_id)): Path<(ShortId, ShortId, ShortId)>,
    Extension(identity): Extension<Identity>,
    Extension(repository): Extension<Repository>,
    request: Json<UpdateNotification>,
) -> Result<Json<Notification>, ApiError> {
    let notification: Notification = repository
        .notification()
        .update(
            &identity,
            project_id.as_uuid(),
            check_id.as_uuid(),
            notification_id.as_uuid(),
            request.0.into(),
        )
        .await?
        .into();
    Ok(notification.into())
}

/// Handler for `DELETE /api/v1/projects/:id/checks/:id/notifications/:id`
pub async fn delete(
    Path((project_id, check_id, notification_id)): Path<(ShortId, ShortId, ShortId)>,
    Extension(identity): Extension<Identity>,
    Extension(repository): Extension<Repository>,
) -> Result<impl IntoResponse, ApiError> {
    repository
        .notification()
        .delete(
            &identity,
            project_id.as_uuid(),
            check_id.as_uuid(),
            notification_id.as_uuid(),
        )
        .await?;
    Ok(Empty::new())
}

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

impl From<NotificationType> for dto::NotificationType {
    fn from(notification_type: NotificationType) -> Self {
        match notification_type {
            NotificationType::Email => dto::NotificationType::Email,
            NotificationType::Webhook => dto::NotificationType::Webhook,
        }
    }
}

impl From<CreateNotification> for dto::CreateNotification {
    fn from(request: CreateNotification) -> Self {
        Self {
            notification_type: request.notification_type.into(),
            name: request.name,
            email: request.email,
            url: request.url,
            max_retries: request.max_retries,
        }
    }
}

impl From<UpdateNotification> for dto::UpdateNotification {
    fn from(request: UpdateNotification) -> Self {
        Self {
            name: request.name,
            notification_type: request.notification_type.map(|t| t.into()),
            email: request.email,
            url: request.url,
            max_retries: request.max_retries,
        }
    }
}
