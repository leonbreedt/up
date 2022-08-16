#![allow(clippy::manual_map)]
sea_query::sea_query_driver_postgres!();

pub use sea_query_driver_postgres::{bind_query, bind_query_as};

use std::{fmt::Debug, hash::Hash, str::FromStr};

use miette::Diagnostic;
use sea_query::{Expr, QueryBuilder, SimpleExpr, Value};
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

use crate::database::{Database, DbQueryBuilder};

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

/// Represents a value to update in a repository.
#[derive(Clone)]
pub enum QueryValue<T: ModelField> {
    Value(T, Value),
    Expression(T, SimpleExpr),
}

impl<T: ModelField> QueryValue<T> {
    pub fn value<V: Into<Value>>(field: T, value: V) -> Self {
        Self::Value(field, value.into())
    }

    pub fn expr<E: Into<SimpleExpr>>(field: T, value: E) -> Self {
        Self::Expression(field, value.into())
    }

    pub fn field(&self) -> &T {
        match self {
            Self::Value(f, _) => f,
            Self::Expression(f, _) => f,
        }
    }

    pub fn to_expr(&self) -> SimpleExpr {
        match self {
            Self::Value(_, v) => Expr::value(v.clone()),
            Self::Expression(_, e) => e.clone(),
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            Self::Value(_, v) => v.clone(),
            Self::Expression(_, e) => match e {
                SimpleExpr::Value(v) => v.clone(),
                _ => panic!("not a single Value expression"),
            },
        }
    }
}

// Support being used by .values() of query builders.
impl<T: ModelField> From<QueryValue<T>> for (T, Value) {
    fn from(v: QueryValue<T>) -> Self {
        (v.field().clone(), v.to_value())
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

pub fn updatable_values<T: ModelField + 'static>(values: Vec<QueryValue<T>>) -> Vec<QueryValue<T>> {
    values
        .into_iter()
        .filter(|v| T::updatable().contains(v.field()))
        .collect()
}
