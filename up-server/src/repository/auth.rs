use std::str::FromStr;
use uuid::Uuid;

use crate::{database::Database, repository::Result};

#[derive(sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub uuid: Uuid,
    pub email: String,
    pub account_ids: Vec<String>,
    pub project_ids: Vec<String>,
    pub roles: Vec<String>,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserRole {
    Administrator,
    Member,
    Viewer,
}

impl FromStr for UserRole {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "ADMINISTRATOR" => Ok(UserRole::Administrator),
            "MEMBER" => Ok(UserRole::Member),
            "VIEWER" => Ok(UserRole::Viewer),
            _ => Err(format!("{} is not a supported role value", s)),
        }
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
                    SELECT DISTINCT ur.role || '|' || ur.account_id
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
