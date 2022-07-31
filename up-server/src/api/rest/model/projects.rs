use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{repository::dto, shortid::ShortId};

#[derive(Debug, Serialize, Deserialize)]
pub struct Create {
    // TODO: remove, this should be part of logged in context
    pub account_id: ShortId,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Update {
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: ShortId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<dto::project::Project> for Project {
    fn from(issue: dto::project::Project) -> Self {
        Self {
            id: issue.uuid.unwrap().into(),
            name: issue.name.unwrap(),
            created_at: issue.created_at.unwrap(),
            updated_at: issue.updated_at,
        }
    }
}
