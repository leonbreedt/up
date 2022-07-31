use std::{collections::HashMap, fmt::Debug, hash::Hash, str::FromStr};

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use sea_query::Iden;
use uuid::Uuid;

use super::ModelField;

#[derive(sqlx::Type)]
#[sqlx(type_name = "schedule_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScheduleType {
    Simple,
    Cron,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "check_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CheckStatus {
    Up,
    Down,
    Created,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "period_units", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PeriodUnits {
    Hours,
    Minutes,
    Days,
}

pub struct Check {
    pub uuid: Option<Uuid>,
    pub ping_key: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<CheckStatus>,
    pub schedule_type: Option<ScheduleType>,
    pub ping_period: Option<i32>,
    pub ping_period_units: Option<PeriodUnits>,
    pub ping_cron_expression: Option<String>,
    pub grace_period: Option<i32>,
    pub grace_period_units: Option<PeriodUnits>,
    pub last_ping_at: Option<DateTime<Utc>>,
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
    PingKey,
    Name,
    Description,
    ScheduleType,
    PingPeriod,
    PingPeriodUnits,
    PingCronExpression,
    GracePeriod,
    GracePeriodUnits,
    Status,
    LastPingAt,
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
        &[
            Field::Name,
            Field::Description,
            Field::ScheduleType,
            Field::PingPeriod,
            Field::PingPeriodUnits,
            Field::PingCronExpression,
            Field::GracePeriod,
            Field::GracePeriodUnits,
            Field::Status,
        ]
    }
}

lazy_static! {
    static ref NAME_TO_FIELD: HashMap<String, Field> = vec![
        (Field::Id.to_string(), Field::Id),
        (Field::AccountId.to_string(), Field::AccountId),
        (Field::ProjectId.to_string(), Field::ProjectId),
        (Field::Uuid.to_string(), Field::Uuid),
        (Field::ShortId.to_string(), Field::ShortId),
        (Field::PingKey.to_string(), Field::PingKey),
        (Field::Name.to_string(), Field::Name),
        (Field::Description.to_string(), Field::Description),
        (Field::ScheduleType.to_string(), Field::ScheduleType),
        (Field::PingPeriod.to_string(), Field::PingPeriod),
        (Field::PingPeriodUnits.to_string(), Field::PingPeriodUnits),
        (
            Field::PingCronExpression.to_string(),
            Field::PingCronExpression
        ),
        (Field::GracePeriod.to_string(), Field::GracePeriod),
        (Field::GracePeriodUnits.to_string(), Field::GracePeriodUnits),
        (Field::Status.to_string(), Field::Status),
        (Field::LastPingAt.to_string(), Field::LastPingAt),
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
            Self::PingKey => "ping_key",
            Self::Name => "name",
            Self::Description => "description",
            Self::ScheduleType => "schedule_type",
            Self::PingPeriod => "ping_period",
            Self::PingPeriodUnits => "ping_period_units",
            Self::PingCronExpression => "ping_cron_expression",
            Self::GracePeriod => "grace_period",
            Self::GracePeriodUnits => "grace_period_units",
            Self::Status => "status",
            Self::LastPingAt => "last_ping_at",
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
