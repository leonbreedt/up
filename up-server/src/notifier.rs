#![allow(dead_code)]

use miette::Diagnostic;
use thiserror::Error;

use crate::repository::Repository;

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

    pub fn send_email(
        address: String,
        subject: String,
        textBody: String,
        htmlBody: String,
    ) -> Result<()> {
        Ok(())
    }
}
