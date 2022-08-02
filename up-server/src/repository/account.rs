use std::{collections::HashMap, fmt::Debug, hash::Hash, str::FromStr};

use lazy_static::lazy_static;
use sea_query::{Expr, Iden, Query, SelectStatement};
use sqlx::Row;
use uuid::Uuid;

use crate::database::DbConnection;
use crate::{
    database::{Database, DbQueryBuilder},
    repository::RepositoryError,
    shortid::ShortId,
};

use super::{bind_query, ModelField, Result};

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

    pub async fn get_account_id(&self, conn: &mut DbConnection, uuid: &Uuid) -> Result<i64> {
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
                entity_type: ENTITY_ACCOUNT.to_string(),
                id: ShortId::from_uuid(uuid).to_string(),
            })
        }
    }
}

mod queries {
    use super::*;

    pub fn read_statement(selected_fields: &[Field]) -> SelectStatement {
        let mut statement = Query::select();

        statement
            .from(Field::Table)
            .columns(selected_fields.to_vec())
            .and_where(Expr::col(Field::Deleted).eq(false));

        statement
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Field {
    Table,
    Id,
    Uuid,
    CreatedAt,
    Deleted,
    DeletedAt,
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
            Self::Table => "accounts",
            Self::Id => "id",
            Self::Uuid => "uuid",
            Self::CreatedAt => "created_at",
            Self::Deleted => "deleted",
            Self::DeletedAt => "deleted_at",
        }
    }
}

lazy_static! {
    static ref NAME_TO_FIELD: HashMap<&'static str, Field> = vec![
        (Field::Id.as_ref(), Field::Id),
        (Field::Uuid.as_ref(), Field::Uuid),
        (Field::CreatedAt.as_ref(), Field::CreatedAt),
        (Field::Deleted.as_ref(), Field::Deleted),
        (Field::DeletedAt.as_ref(), Field::DeletedAt),
    ]
    .into_iter()
    .collect();
}

impl FromStr for Field {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        if let Some(field) = NAME_TO_FIELD.get(value) {
            Ok(field.clone())
        } else {
            anyhow::bail!("unsupported Accounts variant '{}'", value);
        }
    }
}
