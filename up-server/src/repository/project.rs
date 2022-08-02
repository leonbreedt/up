use std::{collections::HashMap, fmt::Debug, fmt::Write as _, hash::Hash, str::FromStr};

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use lazy_static::lazy_static;
use sea_query::{
    Expr, Iden, InsertStatement, Query, QueryBuilder, SelectStatement, UpdateStatement,
};
use sqlx::Row;
use tracing::Level;
use uuid::Uuid;

use super::{bind_query, maybe_field_value};
use crate::database::DbConnection;
use crate::repository::account::AccountRepository;
use crate::{
    database::{Database, DbQueryBuilder, DbRow},
    repository::{RepositoryError, Result},
    shortid::ShortId,
};

use super::ModelField;

const ENTITY_PROJECT: &str = "project";

#[derive(Clone)]
pub struct ProjectRepository {
    database: Database,
    account: AccountRepository,
}

impl ProjectRepository {
    pub fn new(database: Database, account: AccountRepository) -> Self {
        Self { database, account }
    }

    pub async fn get_project_id(&self, conn: &mut DbConnection, uuid: &Uuid) -> Result<i64> {
        let (sql, params) = queries::read_statement(&[Field::Id])
            .and_where(Expr::col(Field::Uuid).eq(*uuid))
            .build(DbQueryBuilder::default());
        let row = bind_query(sqlx::query(&sql), &params)
            .fetch_optional(&mut *conn)
            .await?;
        if let Some(row) = row {
            Ok(row.try_get("id")?)
        } else {
            Err(RepositoryError::NotFound {
                entity_type: ENTITY_PROJECT.to_string(),
                id: ShortId::from_uuid(uuid).to_string(),
            })
        }
    }

    pub async fn read_one_project(&self, select_fields: &[Field], uuid: &Uuid) -> Result<Project> {
        let mut conn = self.database.connection().await?;
        queries::read_one(&mut conn, select_fields, uuid).await
    }

    pub async fn read_projects(&self, select_fields: &[Field]) -> Result<Vec<Project>> {
        let mut conn = self.database.connection().await?;
        queries::read_all(&mut conn, select_fields).await
    }

    pub async fn create_project(
        &self,
        select_fields: &[Field],
        account_uuid: &Uuid,
        name: &str,
    ) -> Result<Project> {
        let mut tx = self.database.transaction().await?;

        let account_id = self.account.get_account_id(&mut tx, account_uuid).await?;
        let project = queries::insert(&mut tx, select_fields, account_id, name).await?;
        let uuid = project.uuid.as_ref().unwrap();

        tx.commit().await?;

        tracing::trace!(
            account_uuid = account_uuid.to_string(),
            uuid = uuid.to_string(),
            name = name,
            "project created"
        );

        Ok(project)
    }

    pub async fn update_project(
        &self,
        uuid: &Uuid,
        select_fields: &[Field],
        update_fields: Vec<(Field, sea_query::Value)>,
    ) -> Result<(bool, Project)> {
        let mut tx = self.database.transaction().await?;

        let (updated, check) = queries::update(&mut tx, uuid, select_fields, update_fields).await?;

        tx.commit().await?;

        if updated {
            tracing::trace!(uuid = uuid.to_string(), "project updated");
        } else {
            tracing::trace!(uuid = uuid.to_string(), "no change, project not updated");
        }

        Ok((updated, check))
    }

    pub async fn delete_project(&self, uuid: &Uuid) -> Result<bool> {
        let mut tx = self.database.transaction().await?;

        let deleted = queries::delete(&mut tx, uuid).await?;

        tx.commit().await?;

        if deleted {
            tracing::trace!(uuid = uuid.to_string(), "project deleted");
        }

        Ok(deleted)
    }
}

pub struct Project {
    pub uuid: Option<Uuid>,
    pub name: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Field {
    Table,
    Id,
    AccountId,
    Uuid,
    ShortId,
    Name,
    CreatedAt,
    UpdatedAt,
    Deleted,
    DeletedAt,
}

impl Field {
    pub fn all() -> &'static [Field] {
        &ALL_FIELDS
    }

    pub fn updatable() -> &'static [Field] {
        &[Field::Name, Field::AccountId]
    }
}

impl ModelField for Field {}

impl Iden for Field {
    fn unquoted(&self, s: &mut dyn std::fmt::Write) {
        write!(s, "{}", self.as_ref()).unwrap();
    }
}

impl AsRef<str> for Field {
    fn as_ref(&self) -> &str {
        match self {
            Self::Table => "projects",
            Self::Id => "id",
            Self::AccountId => "account_id",
            Self::Uuid => "uuid",
            Self::ShortId => "shortid",
            Self::Name => "name",
            Self::CreatedAt => "created_at",
            Self::UpdatedAt => "updated_at",
            Self::Deleted => "deleted",
            Self::DeletedAt => "deleted_at",
        }
    }
}

lazy_static! {
    static ref NAME_TO_FIELD: HashMap<&'static str, Field> = vec![
        (Field::Id.as_ref(), Field::Id),
        (Field::AccountId.as_ref(), Field::AccountId),
        (Field::Uuid.as_ref(), Field::Uuid),
        (Field::ShortId.as_ref(), Field::ShortId),
        (Field::Name.as_ref(), Field::Name),
        (Field::CreatedAt.as_ref(), Field::CreatedAt),
        (Field::UpdatedAt.as_ref(), Field::UpdatedAt),
        (Field::Deleted.as_ref(), Field::Deleted),
        (Field::DeletedAt.as_ref(), Field::DeletedAt),
    ]
    .into_iter()
    .collect();
    static ref ALL_FIELDS: Vec<Field> = NAME_TO_FIELD.values().cloned().collect();
}

impl FromStr for Field {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        if let Some(field) = NAME_TO_FIELD.get(value) {
            Ok(field.clone())
        } else {
            anyhow::bail!("unsupported Project variant '{}'", value);
        }
    }
}

mod queries {
    use super::*;
    use crate::database::DbConnection;

    pub async fn read_one(
        conn: &mut DbConnection,
        select_fields: &[Field],
        uuid: &Uuid,
    ) -> Result<Project> {
        tracing::trace!(
            select = format!("{:?}", select_fields),
            uuid = uuid.to_string(),
            "reading project"
        );

        let (sql, params) = read_statement(select_fields)
            .and_where(Expr::col(Field::Uuid).eq(*uuid))
            .build(DbQueryBuilder::default());

        bind_query(sqlx::query(&sql), &params)
            .fetch_optional(&mut *conn)
            .await?
            .map(|row| from_row(&row, select_fields))
            .ok_or_else(|| RepositoryError::NotFound {
                entity_type: ENTITY_PROJECT.to_string(),
                id: ShortId::from_uuid(uuid).to_string(),
            })?
    }

    pub async fn read_all(
        conn: &mut DbConnection,
        select_fields: &[Field],
    ) -> Result<Vec<Project>> {
        tracing::trace!(
            select = format!("{:?}", select_fields),
            "reading all projects"
        );

        let (sql, params) = read_statement(select_fields).build(DbQueryBuilder::default());

        bind_query(sqlx::query(&sql), &params)
            .fetch_all(&mut *conn)
            .await?
            .into_iter()
            .map(|row| from_row(&row, select_fields))
            .collect()
    }

    pub async fn insert(
        conn: &mut DbConnection,
        select_fields: &[Field],
        account_id: i64,
        name: &str,
    ) -> Result<Project> {
        tracing::trace!(
            select = format!("{:?}", select_fields),
            account_id = account_id.to_string(),
            name = name,
            "creating project"
        );

        let (sql, params) =
            insert_statement(select_fields, account_id, name)?.build(DbQueryBuilder::default());

        let row = bind_query(sqlx::query(&sql), &params)
            .fetch_one(&mut *conn)
            .await?;
        let issue = from_row(&row, select_fields)?;

        Ok(issue)
    }

    pub async fn update(
        conn: &mut DbConnection,
        uuid: &Uuid,
        select_fields: &[Field],
        update_fields: Vec<(Field, sea_query::Value)>,
    ) -> Result<(bool, Project)> {
        let update_params: Vec<(Field, sea_query::Value)> = update_fields
            .into_iter()
            .filter(|i| Field::updatable().contains(&i.0))
            .collect();

        let query_builder = DbQueryBuilder::default();

        if tracing::event_enabled!(Level::TRACE) {
            let mut fields_to_update = String::from("[");
            for field in update_params.iter() {
                let _ = write!(
                    fields_to_update,
                    "{}={}",
                    field.0.as_ref(),
                    query_builder.value_to_string(&field.1)
                );
            }
            fields_to_update.push(']');
            tracing::trace!(
                uuid = uuid.to_string(),
                fields = fields_to_update,
                "updating check"
            );
        }

        let mut updated = false;
        if !update_params.is_empty() {
            let (sql, params) = update_statement(&update_params)
                .and_where(Expr::col(Field::Uuid).eq(*uuid))
                .and_where(Expr::col(Field::Deleted).eq(false))
                .build(query_builder);

            let rows_updated = bind_query(sqlx::query(&sql), &params)
                .execute(&mut *conn)
                .await?
                .rows_affected();

            updated = rows_updated > 0
        }

        let check = read_one(&mut *conn, select_fields, uuid).await?;
        Ok((updated, check))
    }

    pub async fn delete(conn: &mut DbConnection, uuid: &Uuid) -> Result<bool> {
        tracing::trace!(uuid = uuid.to_string(), "deleting project");

        let (sql, params) = update_statement(&[
            (Field::Deleted, true.into()),
            (Field::DeletedAt, Utc::now().into()),
        ])
        .and_where(Expr::col(Field::Uuid).eq(*uuid))
        .build(DbQueryBuilder::default());

        let rows_deleted = bind_query(sqlx::query(&sql), &params)
            .execute(&mut *conn)
            .await?
            .rows_affected();

        Ok(rows_deleted > 0)
    }

    pub fn read_statement(selected_fields: &[Field]) -> SelectStatement {
        let mut statement = Query::select();

        statement
            .from(Field::Table)
            .columns(selected_fields.to_vec())
            .and_where(Expr::col(Field::Deleted).eq(false));

        statement
    }

    fn insert_statement(
        select_fields: &[Field],
        account_id: i64,
        name: &str,
    ) -> Result<InsertStatement> {
        let mut statement = Query::insert();

        let now = Utc::now();
        let id = Uuid::new_v4();
        let short_id: ShortId = id.into();

        statement
            .into_table(Field::Table)
            .columns([
                Field::AccountId,
                Field::Uuid,
                Field::ShortId,
                Field::Name,
                Field::CreatedAt,
                Field::UpdatedAt,
            ])
            .values(vec![
                account_id.into(),
                id.into(),
                short_id.into(),
                name.into(),
                now.into(),
                now.into(),
            ])?
            .returning(Query::returning().columns(select_fields.to_vec()));

        Ok(statement)
    }

    fn update_statement(values: &[(Field, sea_query::Value)]) -> UpdateStatement {
        let mut statement = Query::update();

        let mut values = values.to_vec();
        values.push((Field::UpdatedAt, Utc::now().into()));

        statement
            .table(Field::Table)
            .values(values)
            .and_where(Expr::col(Field::Deleted).eq(false));

        statement
    }

    fn from_row(row: &DbRow, select_fields: &[Field]) -> Result<Project> {
        let created_at: Option<NaiveDateTime> =
            maybe_field_value(row, select_fields, &Field::CreatedAt)?;
        let updated_at: Option<NaiveDateTime> =
            maybe_field_value(row, select_fields, &Field::UpdatedAt)?;
        let uuid: Option<Uuid> = maybe_field_value(row, select_fields, &Field::Uuid)?;
        Ok(Project {
            uuid,
            name: maybe_field_value(row, select_fields, &Field::Name)?,
            created_at: created_at.map(|v| Utc.from_utc_datetime(&v)),
            updated_at: updated_at.map(|v| Utc.from_utc_datetime(&v)),
        })
    }
}
