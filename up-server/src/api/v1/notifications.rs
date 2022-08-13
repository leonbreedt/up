use axum::{body::Empty, extract::Path, response::IntoResponse, Extension};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::repository::{column_expression, column_value, QueryValue};
use crate::{
    api::json::Json,
    api::v1::ApiError,
    repository::{dto, Repository},
    shortid::ShortId,
};

/// Handler for `GET /api/v1/notifications/:id`
pub async fn read_one(
    Path(id): Path<ShortId>,
    repository: Extension<Repository>,
) -> Result<Json<Notification>, ApiError> {
    let notification: Notification = repository
        .notification()
        .read_one(id.as_uuid())
        .await?
        .into();
    Ok(notification.into())
}

/// Handler for `GET /api/v1/notifications`
pub async fn read_all(
    Extension(repository): Extension<Repository>,
) -> Result<Json<Vec<Notification>>, ApiError> {
    let notifications: Vec<Notification> = repository
        .notification()
        .read_all()
        .await?
        .into_iter()
        .map(|i| i.into())
        .collect();
    Ok(notifications.into())
}

/// Handler for `POST /api/v1/notifications`
pub async fn create(
    repository: Extension<Repository>,
    request: Json<CreateNotification>,
) -> Result<Json<Notification>, ApiError> {
    let notification = repository
        .notification()
        .create(
            request.account_id.as_uuid(),
            request.project_id.as_uuid(),
            request.values(),
        )
        .await?;
    let notification: Notification = notification.into();
    Ok(notification.into())
}

/// Handler for `PATCH /api/v1/notifications/:id`
pub async fn update(
    Path(id): Path<ShortId>,
    repository: Extension<Repository>,
    request: Json<UpdateNotification>,
) -> Result<Json<Notification>, ApiError> {
    let (_, notification) = repository
        .notification()
        .update(id.as_uuid(), request.values())
        .await?;
    let notification: Notification = notification.into();
    Ok(notification.into())
}

/// Handler for `DELETE /api/v1/notifications/:id`
pub async fn delete(
    Path(id): Path<ShortId>,
    Extension(repository): Extension<Repository>,
) -> Result<impl IntoResponse, ApiError> {
    repository.notification().delete(id.as_uuid()).await?;
    Ok(Empty::new())
}

// API model types

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
    pub account_id: ShortId,
    pub project_id: ShortId,
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

// Model conversions

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
