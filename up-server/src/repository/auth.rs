use uuid::Uuid;

use crate::{database::Database, repository::Result};

#[derive(sqlx::FromRow)]
pub struct User {
    pub uuid: Uuid,
    pub email: String,
}

#[derive(Clone)]
pub struct AuthRepository {
    database: Database,
}

impl AuthRepository {
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn find_user_by_subject(&self, subject: &str) -> Result<Option<User>> {
        let mut conn = self.database.connection().await?;

        let sql = r"
            SELECT
                uuid,
                email
            FROM
                users
            WHERE
                subject = $1
                AND
                deleted = false            
        ";

        let user: Option<User> = sqlx::query_as(sql)
            .bind(subject)
            .fetch_optional(&mut conn)
            .await?;

        Ok(user)
    }
}
