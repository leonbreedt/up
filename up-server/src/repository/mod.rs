#![allow(clippy::manual_map)]
sea_query::sea_query_driver_postgres!();

pub use sea_query_driver_postgres::{bind_query, bind_query_as};

use std::{fmt::Debug, hash::Hash, str::FromStr};

use miette::Diagnostic;
use sea_query::{Expr, QueryBuilder, SimpleExpr, Value};
use sqlx::{Row, ValueRef};
use thiserror::Error;

mod account;
mod check;
mod notification;
mod project;

pub mod dto {
    pub use super::check::{Check, CheckStatus, Field as CheckField, PeriodUnits, ScheduleType};
    pub use super::notification::{
        Field as NotificationField, Notification, NotificationAlert, NotificationType,
    };
    pub use super::project::{Field as ProjectField, Project};
}

use account::AccountRepository;
use check::CheckRepository;
use notification::NotificationRepository;
use project::ProjectRepository;

use crate::database::{Database, DbQueryBuilder, DbRow, DbType};

type Result<T> = miette::Result<T, RepositoryError>;

/// Represents a field in a DTO (can be used in queries, parse from
/// strings, converted to strings, and used as map keys).
pub trait ModelField: Debug + Clone + Hash + PartialEq + Eq + FromStr + AsRef<str> {}

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
    #[error("check with ping key '{key}' does not exist")]
    #[diagnostic(code(up::error::bad_argument))]
    NotFoundPingKey { key: String },
    #[error("SQL query failed")]
    #[diagnostic(code(up::error::sql))]
    SqlQueryFailed(#[from] sqlx::Error),
    #[error("failed to build SQL query")]
    #[diagnostic(code(up::error::sql_query))]
    BuildSqlQueryFailed(#[from] sea_query::error::Error),
    #[error("failed to execute background task")]
    #[diagnostic(code(up::error::background_task))]
    BackgroundTaskFailed(#[from] tokio::task::JoinError),
}

impl Repository {
    pub fn new(database: Database) -> Self {
        let account = AccountRepository::new(database.clone());
        let project = ProjectRepository::new(database.clone(), account.clone());
        let check = CheckRepository::new(database.clone(), account, project.clone());
        let notification = NotificationRepository::new(database, check.clone());
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

pub(crate) fn maybe_field_value<'r, F, V>(
    row: &'r DbRow,
    selection: &[F],
    field: &F,
) -> Result<Option<V>>
where
    F: ModelField,
    V: sqlx::Decode<'r, DbType> + sqlx::Type<DbType>,
{
    if selection.contains(field) {
        let index = field.as_ref();
        let value_ref = row.try_get_raw(index)?;
        if value_ref.is_null() {
            Ok(None)
        } else {
            Ok(Some(row.try_get(index)?))
        }
    } else {
        Ok(None)
    }
}

/// Represents a value to update in a repository.
#[derive(Clone)]
pub enum QueryValue<T: ModelField> {
    Value(T, Value),
    Expression(T, SimpleExpr),
}

impl<T: ModelField> QueryValue<T> {
    pub fn field(&self) -> &T {
        match self {
            Self::Value(f, _) => f,
            Self::Expression(f, _) => f,
        }
    }

    pub fn as_expr(&self) -> SimpleExpr {
        match self {
            Self::Value(_, v) => Expr::value(v.clone()),
            Self::Expression(_, e) => e.clone(),
        }
    }
}

impl<T: ModelField> ToString for QueryValue<T> {
    fn to_string(&self) -> String {
        match self {
            Self::Value(_f, v) => DbQueryBuilder::default().value_to_string(v),
            Self::Expression(_f, v) => format!("{:?}", v),
        }
    }
}

pub fn column_value<F, V>(field: F, value: V) -> QueryValue<F>
where
    F: ModelField,
    V: Into<Value>,
{
    QueryValue::Value(field, value.into())
}

pub fn column_expression<F, E>(field: F, value: E) -> QueryValue<F>
where
    F: ModelField,
    E: Into<SimpleExpr>,
{
    QueryValue::Expression(field, value.into())
}
