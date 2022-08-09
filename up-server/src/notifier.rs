#![allow(dead_code)]

use miette::Diagnostic;
use thiserror::Error;

use crate::repository::{dto::OverdueCheck, Repository};

#[derive(Clone)]
pub struct Notifier {
    repository: Repository,
}

type Result<T> = miette::Result<T, NotifierError>;

#[derive(Error, Diagnostic, Debug)]
pub enum NotifierError {}

impl Notifier {
    pub fn with_repository(repository: Repository) -> Self {
        Self { repository }
    }

    pub async fn send_overdue_check_notification(&self, check: &OverdueCheck) {
        tracing::debug!(
            "overdue: {:?} (status={}, last_pinged_at={:?}, email={})",
            check.inner.uuid,
            check.inner.status.to_string(),
            check.inner.last_ping_at,
            check.email
        );
    }
}
