use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{repository::dto, shortid::ShortId};

#[derive(Debug, Serialize, Deserialize)]
pub struct Create {
    // TODO: remove, this should be part of logged in context
    pub account_id: ShortId,
    pub project_id: ShortId,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Update {
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CheckStatus {
    Up,
    Down,
    Created,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScheduleType {
    Simple,
    Cron,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PeriodUnits {
    Minutes,
    Hours,
    Days,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Check {
    pub id: ShortId,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: CheckStatus,
    pub schedule_type: ScheduleType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ping_period: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ping_period_units: Option<PeriodUnits>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ping_cron_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period_units: Option<PeriodUnits>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_ping_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<dto::check::CheckStatus> for CheckStatus {
    fn from(status: dto::check::CheckStatus) -> Self {
        match status {
            dto::check::CheckStatus::Up => CheckStatus::Up,
            dto::check::CheckStatus::Down => CheckStatus::Down,
            dto::check::CheckStatus::Created => CheckStatus::Created,
        }
    }
}

impl From<dto::check::ScheduleType> for ScheduleType {
    fn from(status: dto::check::ScheduleType) -> Self {
        match status {
            dto::check::ScheduleType::Simple => ScheduleType::Simple,
            dto::check::ScheduleType::Cron => ScheduleType::Cron,
        }
    }
}

impl From<dto::check::PeriodUnits> for PeriodUnits {
    fn from(status: dto::check::PeriodUnits) -> Self {
        match status {
            dto::check::PeriodUnits::Minutes => PeriodUnits::Minutes,
            dto::check::PeriodUnits::Hours => PeriodUnits::Hours,
            dto::check::PeriodUnits::Days => PeriodUnits::Days,
        }
    }
}

impl From<dto::check::Check> for Check {
    fn from(issue: dto::check::Check) -> Self {
        Self {
            id: issue.uuid.unwrap().into(),
            name: issue.name.unwrap(),
            description: issue.description,
            status: issue.status.unwrap().into(),
            schedule_type: issue.schedule_type.unwrap().into(),
            ping_period: issue.ping_period,
            ping_period_units: issue.ping_period_units.map(|u| u.into()),
            ping_cron_expression: issue.ping_cron_expression,
            grace_period: issue.grace_period,
            grace_period_units: issue.grace_period_units.map(|u| u.into()),
            last_ping_at: issue.last_ping_at,
            created_at: issue.created_at.unwrap(),
            updated_at: issue.updated_at,
        }
    }
}
