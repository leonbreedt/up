#![allow(clippy::manual_map)]
sea_query::sea_query_driver_postgres!();

pub use sea_query_driver_postgres::bind_query;

use miette::Diagnostic;
use sqlx::{Row, ValueRef};
use std::fmt::Debug;
use std::hash::Hash;
use std::str::FromStr;
use thiserror::Error;

mod account;
mod check;
mod project;

pub mod dto {
    pub use super::check::{Check, CheckStatus, Field as CheckField, PeriodUnits, ScheduleType};
    pub use super::project::{Field as ProjectField, Project};
}

use account::AccountRepository;
use check::CheckRepository;
use project::ProjectRepository;

use crate::database::{Database, DbRow, DbType};

type Result<T> = miette::Result<T, RepositoryError>;

/// Represents a field in a DTO (can be used in queries, parse from
/// strings, converted to strings, and used as map keys).
pub trait ModelField: Debug + Clone + Hash + PartialEq + Eq + FromStr + AsRef<str> {}

#[derive(Clone)]
pub struct Repository {
    check: CheckRepository,
    project: ProjectRepository,
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
}

impl Repository {
    pub fn new(database: Database) -> Self {
        let account = AccountRepository::new(database.clone());
        let project = ProjectRepository::new(database.clone(), account.clone());
        let check = CheckRepository::new(database, account, project.clone());
        Self { check, project }
    }

    pub fn check(&self) -> &CheckRepository {
        &self.check
    }

    pub fn project(&self) -> &ProjectRepository {
        &self.project
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
