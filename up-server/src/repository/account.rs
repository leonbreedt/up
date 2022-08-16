use std::{collections::HashMap, fmt::Debug, hash::Hash, str::FromStr};

use lazy_static::lazy_static;
use sea_query::{Expr, Iden, Query};
use uuid::Uuid;

use crate::database::DbConnection;
use crate::{
    database::{Database, DbQueryBuilder},
    repository::RepositoryError,
    shortid::ShortId,
};

use super::{bind_query_as, ModelField, Result};

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
        let (sql, params) = Query::select()
            .from(Field::Table)
            .columns(vec![Field::Id])
            .and_where(Expr::col(Field::Deleted).eq(false))
            .and_where(Expr::col(Field::Uuid).eq(*uuid))
            .build(DbQueryBuilder::default());

        let result: Option<(i64,)> = bind_query_as(sqlx::query_as(&sql), &params)
            .fetch_optional(&mut *conn)
            .await?;
        if let Some(result) = result {
            Ok(result.0)
        } else {
            Err(RepositoryError::NotFound {
                entity_type: ENTITY_ACCOUNT.to_string(),
                id: ShortId::from_uuid(uuid).to_string(),
            })
        }
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

impl ModelField for Field {
    fn all() -> &'static [Field] {
        &ALL_FIELDS
    }

    fn updatable() -> &'static [Field] {
        &[]
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
    static ref ALL_FIELDS: Vec<Field> = NAME_TO_FIELD.values().cloned().collect();
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
