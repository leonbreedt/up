use std::{fmt::Debug, hash::Hash, str::FromStr};

use miette::Diagnostic;
use thiserror::Error;

mod check;
mod notification;
mod project;

pub mod dto {
    pub use super::check::{
        Check, CheckStatus, CreateCheck, PeriodUnits, ScheduleType, UpdateCheck,
    };
    pub use super::notification::{
        CreateNotification, Notification, NotificationAlert, NotificationType, UpdateNotification,
    };
    pub use super::project::{CreateProject, Project, UpdateProject};
}

use check::CheckRepository;
use notification::NotificationRepository;
use project::ProjectRepository;

use crate::database::Database;

type Result<T> = miette::Result<T, RepositoryError>;

/// Represents a field in a DTO (can be used in queries, parse from
/// strings, converted to strings, and used as map keys).
pub trait ModelField: Debug + Clone + Hash + PartialEq + Eq + FromStr + AsRef<str> {
    fn all() -> &'static [Self];
    fn updatable() -> &'static [Self];
}

#[derive(Clone)]
pub struct Repository {
    check: CheckRepository,
    project: ProjectRepository,
    notification: NotificationRepository,
}

#[derive(Error, Diagnostic, Debug)]
pub enum RepositoryError {
    #[error("{entity_type} does not exist")]
    #[diagnostic(code(up::error::bad_argument))]
    NotFound { entity_type: String, id: String },
    #[error("SQL query failed")]
    #[diagnostic(code(up::error::sql))]
    SqlQueryFailed(#[from] sqlx::Error),
    #[error("failed to execute background task")]
    #[diagnostic(code(up::error::background_task))]
    BackgroundTaskFailed(#[from] tokio::task::JoinError),
}

impl Repository {
    pub fn new(database: Database) -> Self {
        let project = ProjectRepository::new(database.clone());
        let check = CheckRepository::new(database.clone());
        let notification = NotificationRepository::new(database);
        Self {
            check,
            project,
            notification,
        }
    }

    pub fn check(&self) -> &CheckRepository {
        &self.check
    }
    pub fn project(&self) -> &ProjectRepository {
        &self.project
    }
    pub fn notification(&self) -> &NotificationRepository {
        &self.notification
    }
}
