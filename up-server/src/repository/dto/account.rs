use std::{collections::HashMap, fmt::Debug, hash::Hash, str::FromStr};

use lazy_static::lazy_static;
use sea_query::Iden;

use super::ModelField;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Field {
    Table,
    Id,
    Uuid,
    Key,
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
            Self::Key => "key",
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
        (Field::Key.as_ref(), Field::Key),
        (Field::CreatedAt.as_ref(), Field::CreatedAt),
        (Field::Deleted.as_ref(), Field::Deleted),
        (Field::DeletedAt.as_ref(), Field::DeletedAt),
    ]
    .into_iter()
    .collect();
}

impl FromStr for Field {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Some(field) = NAME_TO_FIELD.get(value) {
            Ok(field.clone())
        } else {
            anyhow::bail!("unsupported Accounts variant '{}'", value);
        }
    }
}
