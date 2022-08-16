use uuid::Uuid;

use crate::{
    database::{Database, DbConnection},
    repository::RepositoryError,
    shortid::ShortId,
};

use super::Result;

const ENTITY_ACCOUNT: &str = "account";

#[derive(Clone)]
pub struct AccountRepository {
    _database: Database,
}

impl AccountRepository {
    pub fn new(database: Database) -> Self {
        Self {
            _database: database,
        }
    }

    pub async fn get_id(&self, conn: &mut DbConnection, uuid: &Uuid) -> Result<i64> {
        let sql = r"
            SELECT
                id
            FROM
                accounts 
            WHERE
                uuid = $1 AND deleted = false
        ";

        sqlx::query_as(sql)
            .bind(uuid)
            .fetch_optional(&mut *conn)
            .await?
            .map(|id: (i64,)| id.0)
            .ok_or(RepositoryError::NotFound {
                entity_type: ENTITY_ACCOUNT.to_string(),
                id: ShortId::from_uuid(uuid).to_string(),
            })
    }
}
