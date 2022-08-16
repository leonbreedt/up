use std::{collections::HashMap, fmt::Debug, hash::Hash, str::FromStr};

use chrono::{NaiveDateTime, Utc};
use lazy_static::lazy_static;
use sea_query::{Expr, Iden, Query};
use uuid::Uuid;

use super::{bind_query, bind_query_as};
use crate::{
    database::{Database, DbConnection, DbQueryBuilder},
    repository::{
        account::AccountRepository, updatable_values, QueryValue, RepositoryError, Result,
    },
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

    pub async fn get_id(&self, conn: &mut DbConnection, uuid: &Uuid) -> Result<i64> {
        let (sql, values) = Query::select()
            .from(Field::Table)
            .columns(vec![Field::Id])
            .and_where(Expr::col(Field::Uuid).eq(*uuid))
            .and_where(Expr::col(Field::Deleted).eq(false))
            .build(DbQueryBuilder::default());

        bind_query_as(sqlx::query_as(&sql), &values)
            .fetch_optional(&mut *conn)
            .await?
            .map(|row: (i64,)| row.0)
            .ok_or(RepositoryError::NotFound {
                entity_type: ENTITY_PROJECT.to_string(),
                id: ShortId::from_uuid(uuid).to_string(),
            })
    }

    pub async fn read_one(&self, uuid: &Uuid) -> Result<Project> {
        let mut conn = self.database.connection().await?;

        let (sql, _) = Query::select()
            .from(Field::Table)
            .columns(vec![Field::Id])
            .and_where(Expr::col(Field::Uuid).eq(*uuid))
            .and_where(Expr::col(Field::Deleted).eq(false))
            .build(DbQueryBuilder::default());

        sqlx::query_as(&sql)
            .fetch_optional(&mut *conn)
            .await?
            .ok_or_else(|| RepositoryError::NotFound {
                entity_type: ENTITY_PROJECT.to_string(),
                id: ShortId::from_uuid(uuid).to_string(),
            })
    }

    pub async fn read_all(&self) -> Result<Vec<Project>> {
        let mut conn = self.database.connection().await?;

        let (sql, _) = Query::select()
            .from(Field::Table)
            .columns(vec![Field::Id])
            .and_where(Expr::col(Field::Deleted).eq(false))
            .build(DbQueryBuilder::default());

        Ok(sqlx::query_as(&sql).fetch_all(&mut *conn).await?)
    }

    pub async fn create(&self, account_uuid: &Uuid, name: &str) -> Result<Project> {
        let mut tx = self.database.transaction().await?;

        let account_id = self.account.get_id(&mut tx, account_uuid).await?;

        let now = Utc::now();
        let id = Uuid::new_v4();
        let short_id: ShortId = id.into();

        let (sql, params) = Query::insert()
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
            .returning(Query::returning().columns(Field::all().to_vec()))
            .build(DbQueryBuilder::default());

        let project: Project = bind_query_as(sqlx::query_as(&sql), &params)
            .fetch_one(&mut tx)
            .await?;

        tx.commit().await?;

        tracing::trace!(
            account_uuid = account_uuid.to_string(),
            uuid = id.to_string(),
            name = name,
            "project created"
        );

        Ok(project)
    }

    pub async fn update(&self, uuid: &Uuid, values: Vec<QueryValue<Field>>) -> Result<Project> {
        let mut tx = self.database.transaction().await?;

        let mut values = updatable_values(values);
        values.insert(0, QueryValue::value(Field::UpdatedAt, Utc::now()));
        let values: Vec<(Field, sea_query::Value)> = values.into_iter().map(|v| v.into()).collect();

        let (sql, params) = Query::update()
            .values(values)
            .and_where(Expr::col(Field::Uuid).eq(*uuid))
            .and_where(Expr::col(Field::Deleted).eq(false))
            .returning(Query::returning().columns(Field::all().to_vec()))
            .build(DbQueryBuilder::default());

        let project = bind_query_as(sqlx::query_as(&sql), &params)
            .fetch_one(&mut tx)
            .await?;

        tx.commit().await?;

        tracing::trace!(uuid = uuid.to_string(), "project updated");

        Ok(project)
    }

    pub async fn delete(&self, uuid: &Uuid) -> Result<bool> {
        let mut tx = self.database.transaction().await?;

        let (sql, params) = Query::update()
            .values(vec![
                (Field::Deleted, true.into()),
                (Field::DeletedAt, Utc::now().into()),
            ])
            .and_where(Expr::col(Field::Uuid).eq(*uuid))
            .returning(Query::returning().columns(Field::all().to_vec()))
            .build(DbQueryBuilder::default());

        let deleted = bind_query(sqlx::query(&sql), &params)
            .execute(&mut tx)
            .await?
            .rows_affected()
            > 0;

        if deleted {
            tracing::trace!(uuid = uuid.to_string(), "project deleted");
        } else {
            tracing::trace!(uuid = uuid.to_string(), "no such project");
        }

        Ok(deleted)
    }
}

#[derive(sqlx::FromRow)]
pub struct Project {
    pub uuid: Uuid,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
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

impl ModelField for Field {
    fn all() -> &'static [Field] {
        &ALL_FIELDS
    }

    fn updatable() -> &'static [Self] {
        &[Field::Name, Field::AccountId]
    }
}

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
