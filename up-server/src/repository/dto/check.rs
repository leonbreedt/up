use std::{collections::HashMap, fmt::Debug, hash::Hash, str::FromStr};

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use sea_query::Iden;
use uuid::Uuid;

use super::ModelField;

pub struct Check {
    pub uuid: Option<Uuid>,
    pub name: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Field {
    Table,
    Id,
    AccountId,
    ProjectId,
    Uuid,
    ShortId,
    Name,
    CreatedAt,
    UpdatedAt,
    Deleted,
    DeletedAt,
}

impl Iden for Field {
    fn unquoted(&self, s: &mut dyn std::fmt::Write) {
        write!(s, "{}", self.as_ref()).unwrap();
    }
}

impl Field {
    pub fn all() -> &'static [Field] {
        &ALL_FIELDS
    }

    pub fn updatable() -> &'static [Field] {
        &[Field::Name, Field::AccountId]
    }
}

lazy_static! {
    static ref NAME_TO_FIELD: HashMap<String, Field> = vec![
        (Field::Id.to_string(), Field::Id),
        (Field::AccountId.to_string(), Field::AccountId),
        (Field::ProjectId.to_string(), Field::ProjectId),
        (Field::Uuid.to_string(), Field::Uuid),
        (Field::ShortId.to_string(), Field::ShortId),
        (Field::Name.to_string(), Field::Name),
        (Field::CreatedAt.to_string(), Field::CreatedAt),
        (Field::UpdatedAt.to_string(), Field::UpdatedAt),
        (Field::Deleted.to_string(), Field::Deleted),
        (Field::DeletedAt.to_string(), Field::DeletedAt),
    ]
    .into_iter()
    .collect();
    static ref ALL_FIELDS: Vec<Field> = NAME_TO_FIELD.values().cloned().collect();
}

impl ModelField for Field {}

impl AsRef<str> for Field {
    fn as_ref(&self) -> &str {
        match self {
            Self::Table => "checks",
            Self::Id => "id",
            Self::AccountId => "account_id",
            Self::ProjectId => "project_id",
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

impl FromStr for Field {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Some(field) = NAME_TO_FIELD.get(value) {
            Ok(*field)
        } else {
            anyhow::bail!("unsupported Check variant '{}'", value);
        }
    }
}
