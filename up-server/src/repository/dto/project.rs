use std::{collections::HashMap, fmt::Debug, hash::Hash, str::FromStr};

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use sea_query::Iden;
use uuid::Uuid;

use super::ModelField;

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

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Some(field) = NAME_TO_FIELD.get(value) {
            Ok(field.clone())
        } else {
            anyhow::bail!("unsupported Project variant '{}'", value);
        }
    }
}
