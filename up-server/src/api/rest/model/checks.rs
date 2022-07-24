use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::repository::dto;

#[derive(Debug, Serialize, Deserialize)]
pub struct Create {
    // TODO: remove, this should be part of logged in context
    pub account_id: Uuid,
    pub project_id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Update {
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Check {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<dto::check::Check> for Check {
    fn from(issue: dto::check::Check) -> Self {
        Self {
            id: issue.uuid.unwrap(),
            name: issue.name.unwrap(),
            created_at: issue.created_at.unwrap(),
            updated_at: issue.updated_at,
        }
    }
}
