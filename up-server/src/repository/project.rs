use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::{
    database::Database,
    repository::{RepositoryError, Result},
    shortid::ShortId,
};

const ENTITY_PROJECT: &str = "project";

#[derive(sqlx::FromRow)]
pub struct Project {
    pub uuid: Uuid,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

pub struct CreateProject {
    pub account_uuid: Uuid,
    pub name: String,
}

pub struct UpdateProject {
    pub name: Option<String>,
}

#[derive(Clone)]
pub struct ProjectRepository {
    database: Database,
}

impl ProjectRepository {
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn read_one(&self, uuid: &Uuid) -> Result<Project> {
        let mut conn = self.database.connection().await?;

        tracing::trace!(uuid = uuid.to_string(), "reading project");

        let sql = r"
            SELECT
                *
            FROM
                projects
            WHERE
                uuid = $1
                AND
                deleted = false
        ";

        sqlx::query_as(sql)
            .bind(uuid)
            .fetch_optional(&mut *conn)
            .await?
            .ok_or_else(|| RepositoryError::NotFound {
                entity_type: ENTITY_PROJECT.to_string(),
                id: ShortId::from_uuid(uuid).to_string(),
            })
    }

    pub async fn read_all(&self) -> Result<Vec<Project>> {
        let mut conn = self.database.connection().await?;

        tracing::trace!("reading projects");

        let sql = r"
            SELECT
                *
            FROM
                projects
            WHERE
                deleted = false
        ";

        Ok(sqlx::query_as(sql).fetch_all(&mut *conn).await?)
    }

    pub async fn create(&self, request: CreateProject) -> Result<Project> {
        let mut tx = self.database.transaction().await?;

        let id = Uuid::new_v4();
        let short_id: ShortId = id.into();

        let sql = r"
            INSERT INTO projects (
                account_id,
                uuid,
                shortid,
                name
            ) VALUES (
                (SELECT id FROM accounts WHERE uuid = $1),
                $2,
                $3,
                $4
            ) RETURNING *
        ";

        let project: Project = sqlx::query_as(sql)
            .bind(&request.account_uuid)
            .bind(id)
            .bind(short_id.to_string())
            .bind(&request.name)
            .fetch_one(&mut tx)
            .await?;

        tx.commit().await?;

        tracing::trace!(
            account_uuid = request.account_uuid.to_string(),
            uuid = id.to_string(),
            name = request.name,
            "project created"
        );

        Ok(project)
    }

    pub async fn update(&self, uuid: &Uuid, request: UpdateProject) -> Result<Project> {
        let mut tx = self.database.transaction().await?;

        let sql = r"
            UPDATE
                projects
            SET
                name = COALESCE($2,name)
            WHERE
                uuid = $1
                AND
                deleted = false
            RETURNING *
        ";

        let project = sqlx::query_as(sql)
            .bind(uuid)
            .bind(request.name.as_ref())
            .fetch_one(&mut tx)
            .await?;

        tx.commit().await?;

        tracing::trace!(uuid = uuid.to_string(), "project updated");

        Ok(project)
    }

    pub async fn delete(&self, uuid: &Uuid) -> Result<bool> {
        let mut tx = self.database.transaction().await?;

        let sql = r"
            UPDATE projects
            SET
                deleted = true,
                deleted_at = NOW() AT TIME ZONE 'UTC'
            WHERE
                uuid = $1
        ";

        let deleted = sqlx::query(sql)
            .bind(uuid)
            .execute(&mut tx)
            .await?
            .rows_affected()
            > 0;

        tx.commit().await?;

        if deleted {
            tracing::trace!(uuid = uuid.to_string(), "project deleted");
        } else {
            tracing::trace!(uuid = uuid.to_string(), "no such project, nothing deleted");
        }

        Ok(deleted)
    }
}
