use thiserror::Error;

use crate::database::Database;

pub mod dto;
mod queries;

use dto::check;

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
        name: &str,
    ) -> Result<check::Check> {
        let check =
            queries::check::insert(self.database.pool(), select_fields, account_uuid, name).await?;
        let uuid = check.uuid.as_ref().unwrap();

        tracing::trace!(
            account_uuid = account_uuid,
            uuid = uuid,
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
}
