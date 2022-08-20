use sqlx::postgres::{PgHasArrayType, PgTypeInfo};
use uuid::Uuid;

use crate::{database::Database, repository::Result};

#[derive(sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub uuid: Uuid,
    pub email: String,
    pub account_ids: Vec<String>,
    pub project_ids: Vec<String>,
    pub roles: Vec<UserRole>,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserRole {
    Administrator,
    Member,
    Viewer,
}

impl PgHasArrayType for UserRole {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_user_role")
    }
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
                id,
                uuid,
                email,
                ARRAY(
                    SELECT DISTINCT a.uuid || '|' || a.id
                    FROM user_accounts ua
                    INNER JOIN accounts a ON a.id = ua.account_id
                    WHERE ua.user_id = users.id
                ) AS account_ids,
                ARRAY(
                    SELECT DISTINCT p.uuid || '|' || p.id
                    FROM user_projects up
                    INNER JOIN projects p ON p.id = up.project_id
                    WHERE up.user_id = users.id
                ) AS project_ids,
                ARRAY(
                    SELECT DISTINCT ur.role
                    FROM user_roles ur
                    WHERE ur.user_id = users.id
                ) AS roles
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
