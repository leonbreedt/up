use thiserror::Error;

use crate::database::Database;

pub mod dto;
mod queries;

use dto::{check, project};

type Result<T> = std::result::Result<T, RepositoryError>;

#[derive(Clone)]
pub struct Repository {
    database: Database,
}

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("invalid argument {0}: {1}")]
    InvalidArgument(String, String),
    #[error("query failed: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("SQL generation failed: {0}")]
    SqlGenerationError(#[from] sea_query::error::Error),
    #[error("not found")]
    NotFound,
}

impl Repository {
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn read_one_check(
        &self,
        select_fields: &[check::Field],
        uuid: &str,
    ) -> Result<check::Check> {
        queries::check::read_one(self.database.pool(), select_fields, uuid).await
    }

    pub async fn read_checks(&self, select_fields: &[check::Field]) -> Result<Vec<check::Check>> {
        queries::check::read_all(self.database.pool(), select_fields).await
    }

    pub async fn create_check(
        &self,
        select_fields: &[check::Field],
        account_uuid: &str,
        project_uuid: &str,
        name: &str,
    ) -> Result<check::Check> {
        let check = queries::check::insert(
            self.database.pool(),
            select_fields,
            account_uuid,
            project_uuid,
            name,
        )
        .await?;
        let uuid = check.uuid.as_ref().unwrap();

        tracing::trace!(
            account_uuid = account_uuid,
            uuid = uuid.to_string(),
            name = name,
            "check created"
        );

        Ok(check)
    }

    pub async fn update_check(
        &self,
        uuid: &str,
        select_fields: &[check::Field],
        update_fields: Vec<(check::Field, sea_query::Value)>,
    ) -> Result<(bool, check::Check)> {
        let (updated, check) =
            queries::check::update(self.database.pool(), uuid, select_fields, update_fields)
                .await?;

        if updated {
            tracing::trace!(uuid = uuid, "check updated");
        } else {
            tracing::trace!(uuid = uuid, "no change, check not updated");
        }

        Ok((updated, check))
    }

    pub async fn delete_check(&self, uuid: &str) -> Result<bool> {
        let deleted = queries::check::delete(self.database.pool(), uuid).await?;

        if deleted {
            tracing::trace!(uuid = uuid, "check deleted");
        }

        Ok(deleted)
    }

    pub async fn read_one_project(
        &self,
        select_fields: &[project::Field],
        uuid: &str,
    ) -> Result<project::Project> {
        queries::project::read_one(self.database.pool(), select_fields, uuid).await
    }

    pub async fn read_projects(
        &self,
        select_fields: &[project::Field],
    ) -> Result<Vec<project::Project>> {
        queries::project::read_all(self.database.pool(), select_fields).await
    }

    pub async fn create_project(
        &self,
        select_fields: &[project::Field],
        account_uuid: &str,
        name: &str,
    ) -> Result<project::Project> {
        let project =
            queries::project::insert(self.database.pool(), select_fields, account_uuid, name)
                .await?;
        let uuid = project.uuid.as_ref().unwrap();

        tracing::trace!(
            account_uuid = account_uuid,
            uuid = uuid.to_string(),
            name = name,
            "project created"
        );

        Ok(project)
    }

    pub async fn update_project(
        &self,
        uuid: &str,
        select_fields: &[project::Field],
        update_fields: Vec<(project::Field, sea_query::Value)>,
    ) -> Result<(bool, project::Project)> {
        let (updated, check) =
            queries::project::update(self.database.pool(), uuid, select_fields, update_fields)
                .await?;

        if updated {
            tracing::trace!(uuid = uuid, "project updated");
        } else {
            tracing::trace!(uuid = uuid, "no change, project not updated");
        }

        Ok((updated, check))
    }

    pub async fn delete_project(&self, uuid: &str) -> Result<bool> {
        let deleted = queries::project::delete(self.database.pool(), uuid).await?;

        if deleted {
            tracing::trace!(uuid = uuid, "project deleted");
        }

        Ok(deleted)
    }
}
