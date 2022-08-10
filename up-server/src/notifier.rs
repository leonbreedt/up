#![allow(dead_code)]

use crate::integrations::postmark::{Body, PostmarkClient, SendEmailRequest};
use miette::Diagnostic;
use thiserror::Error;

use crate::repository::{dto::OverdueCheck, Repository};

#[derive(Clone)]
pub struct Notifier {
    repository: Repository,
    postmark_client: PostmarkClient,
}

type Result<T> = miette::Result<T, NotifierError>;

#[derive(Error, Diagnostic, Debug)]
pub enum NotifierError {}

impl Notifier {
    pub fn new(repository: Repository, postmark_client: PostmarkClient) -> Self {
        Self {
            repository,
            postmark_client,
        }
    }

    pub async fn send_overdue_check_notification(&self, check: &OverdueCheck) {
        let last_ping_at = check
            .inner
            .last_ping_at
            .map(|dt| dt.to_string())
            .unwrap_or_else(String::new);

        tracing::debug!(
            check = check.inner.uuid.to_string(),
            last_ping_at = last_ping_at,
            email = check.email.to_string(),
            "ping overdue, sending notification",
        );

        let mut email = SendEmailRequest::default();
        email.from = "up.io <no-reply@sector42.io>".to_string();
        email.to = check.email.to_string();
        email.subject = Some(format!("[DOWN] {}", check.inner.name));
        email.body = Body::Text(String::from("Sent by up.io"));

        if let Err(e) = self.postmark_client.send_email(&email).await {
            tracing::error!(
                check = check.inner.uuid.to_string(),
                last_ping_at = last_ping_at,
                email = check.email.to_string(),
                "failed to send email notification: {}",
                e
            );
        }
    }
}
