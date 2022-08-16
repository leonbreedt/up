#![allow(dead_code)]

use crate::integrations::postmark::{Body, PostmarkClient, PostmarkError, SendEmailRequest};
use chrono::{TimeZone, Utc};
use miette::Diagnostic;
use thiserror::Error;

use crate::repository::dto::NotificationType;
use crate::repository::{dto::NotificationAlert, Repository};

#[derive(Clone)]
pub struct Notifier {
    repository: Repository,
    postmark_client: PostmarkClient,
}

type Result<T> = miette::Result<T, NotifierError>;

#[derive(Error, Diagnostic, Debug)]
pub enum NotifierError {
    #[error("failed to send email notification")]
    #[diagnostic(code(up::error::notification::email))]
    EmailSendError(#[from] PostmarkError),
}

impl Notifier {
    pub fn new(repository: Repository, postmark_client: PostmarkClient) -> Self {
        Self {
            repository,
            postmark_client,
        }
    }

    pub async fn send_alert(&self, alert: &NotificationAlert) -> Result<()> {
        match alert.notification_type {
            NotificationType::Email => self.send_alert_email(alert).await,
            NotificationType::Webhook => self.call_alert_webhook(alert).await,
        }
    }

    async fn call_alert_webhook(&self, alert: &NotificationAlert) -> Result<()> {
        let last_ping_at = alert
            .last_ping_at
            .map(|dt| Utc.from_utc_datetime(&dt))
            .map(|dt| dt.to_string())
            .unwrap_or_else(String::new);
        let webhook_url = alert.url.as_deref().unwrap();

        tracing::debug!(
            check_uuid = alert.check_uuid.to_string(),
            last_ping_at = last_ping_at,
            url = webhook_url,
            "sending alert",
        );

        Ok(())
    }

    async fn send_alert_email(&self, alert: &NotificationAlert) -> Result<()> {
        let last_ping_at = alert
            .last_ping_at
            .map(|dt| Utc.from_utc_datetime(&dt))
            .map(|dt| dt.to_string())
            .unwrap_or_else(String::new);
        let alert_email = alert.email.as_deref().unwrap();

        tracing::debug!(
            check_uuid = alert.check_uuid.to_string(),
            last_ping_at = last_ping_at,
            email = alert_email,
            "sending alert",
        );

        let email = SendEmailRequest {
            from: "up.io <no-reply@sector42.io>".to_string(),
            to: alert_email.to_string(),
            subject: Some(format!("[DOWN] {}", alert.name)),
            body: Body::Text(String::from("Sent by up.io")),
            ..SendEmailRequest::default()
        };

        self.postmark_client.send_email(&email).await?;

        Ok(())
    }
}
