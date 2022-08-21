use std::{borrow::Cow, fmt::Debug};

use miette::Diagnostic;
use thiserror::Error;
use uuid::Uuid;

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

use crate::{
    database::{Database, DbConnection},
    repository::{
        check::ENTITY_CHECK,
        project::{ENTITY_ACCOUNT, ENTITY_PROJECT},
    },
    shortid::ShortId,
};

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

async fn get_project_account_id(
    conn: &mut DbConnection,
    project_uuid: &Uuid,
    account_ids: &[i64],
) -> Result<(i64, i64)> {
    let sql = r"
            SELECT
                id,
                account_id
            FROM
                projects
            WHERE
                uuid = $1
                AND
                account_id = ANY($2)
                AND
                deleted = false
            LIMIT 1
        ";

    let ids: Option<(i64, i64)> = sqlx::query_as(sql)
        .bind(project_uuid)
        .bind(account_ids)
        .fetch_optional(conn)
        .await?;

    ids.ok_or(RepositoryError::NotFound {
        entity_type: ENTITY_PROJECT.to_string(),
        id: ShortId::from(project_uuid).to_string(),
    })
}

async fn get_check_account_id(
    conn: &mut DbConnection,
    check_uuid: &Uuid,
    project_id: i64,
    account_ids: &[i64],
) -> Result<(i64, i64)> {
    let sql = r"
            SELECT
                id,
                account_id
            FROM
                checks
            WHERE
                uuid = $1
                AND
                project_id = $2
                AND
                account_id = ANY($3)
                AND
                deleted = false
            LIMIT 1
        ";

    let ids: Option<(i64, i64)> = sqlx::query_as(sql)
        .bind(check_uuid)
        .bind(project_id)
        .bind(account_ids)
        .fetch_optional(conn)
        .await?;

    ids.ok_or(RepositoryError::NotFound {
        entity_type: ENTITY_CHECK.to_string(),
        id: ShortId::from(check_uuid).to_string(),
    })
}

async fn get_account_id(
    conn: &mut DbConnection,
    account_uuid: &Uuid,
    account_ids: &[i64],
) -> Result<i64> {
    let sql = r"
            SELECT
                id
            FROM
                accounts
            WHERE
                uuid = $1
                AND
                id = ANY($2)
                AND
                deleted = false
            LIMIT 1
        ";

    let ids: Option<(i64,)> = sqlx::query_as(sql)
        .bind(account_uuid)
        .bind(account_ids)
        .fetch_optional(conn)
        .await?;

    ids.map(|id| id.0).ok_or(RepositoryError::NotFound {
        entity_type: ENTITY_ACCOUNT.to_string(),
        id: ShortId::from(account_uuid).to_string(),
    })
}
