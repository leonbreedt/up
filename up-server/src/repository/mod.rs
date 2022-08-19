use std::borrow::Cow;
use std::fmt::Debug;

use miette::Diagnostic;
use thiserror::Error;

mod auth;
mod check;
mod notification;
mod project;

pub mod dto {
    pub use super::auth::{User, UserRole};
    pub use super::check::{
        Check, CheckStatus, CreateCheck, PeriodUnits, ScheduleType, UpdateCheck,
    };
    pub use super::notification::{
        CreateNotification, Notification, NotificationAlert, NotificationType, UpdateNotification,
    };
    pub use super::project::{CreateProject, Project, UpdateProject};
}

use auth::AuthRepository;
use check::CheckRepository;
use notification::NotificationRepository;
use project::ProjectRepository;

use crate::database::Database;

type Result<T> = miette::Result<T, RepositoryError>;

#[derive(Clone)]
pub struct Repository {
    auth: AuthRepository,
    check: CheckRepository,
    project: ProjectRepository,
    notification: NotificationRepository,
}

#[derive(Error, Diagnostic, Debug)]
pub enum RepositoryError {
    #[error("{entity_type} does not exist")]
    #[diagnostic(code(up::error::bad_argument))]
    NotFound { entity_type: String, id: String },
    #[error("permission denied")]
    #[diagnostic(code(up::error::permission))]
    Forbidden,
    #[error("SQL query failed")]
    #[diagnostic(code(up::error::sql))]
    SqlQueryFailed(#[from] sqlx::Error),
    #[error("failed to execute background task")]
    #[diagnostic(code(up::error::background_task))]
    BackgroundTaskFailed(#[from] tokio::task::JoinError),
}

impl RepositoryError {
    pub fn database_error_code(&self) -> Option<Cow<str>> {
        if let RepositoryError::SqlQueryFailed(e) = self {
            return e.as_database_error().and_then(|dbe| dbe.code());
        }
        None
    }

    pub fn is_unique_constraint_violation(&self) -> bool {
        if let Some(code) = self.database_error_code() {
            code == "23505"
        } else {
            false
        }
    }
}

impl Repository {
    pub fn new(database: Database) -> Self {
        let auth = AuthRepository::new(database.clone());
        let project = ProjectRepository::new(database.clone());
        let check = CheckRepository::new(database.clone());
        let notification = NotificationRepository::new(database);
        Self {
            auth,
            check,
            project,
            notification,
        }
    }

    pub fn auth(&self) -> &AuthRepository {
        &self.auth
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
