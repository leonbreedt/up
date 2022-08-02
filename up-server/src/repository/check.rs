use std::{collections::HashMap, fmt::Debug, fmt::Write as _, hash::Hash, str::FromStr};

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use lazy_static::lazy_static;
use sea_query::{
    Expr, Iden, InsertStatement, Query, QueryBuilder, SelectStatement, UpdateStatement,
};
use tracing::Level;
use uuid::Uuid;

use super::{bind_query, maybe_field_value, ModelField};
use crate::{
    database::{Database, DbPool, DbQueryBuilder, DbRow},
    repository::{account::AccountRepository, project::ProjectRepository, RepositoryError, Result},
    shortid::ShortId,
};

const ENTITY_CHECK: &str = "check";

#[derive(Clone)]
pub struct CheckRepository {
    database: Database,
    account: AccountRepository,
    project: ProjectRepository,
}

impl CheckRepository {
    pub fn new(database: Database, account: AccountRepository, project: ProjectRepository) -> Self {
        Self {
            database,
            account,
            project,
        }
    }

    pub async fn read_one_check(&self, select_fields: &[Field], uuid: &Uuid) -> Result<Check> {
        queries::read_one(self.database.pool(), select_fields, uuid).await
    }

    pub async fn read_checks(&self, select_fields: &[Field]) -> Result<Vec<Check>> {
        queries::read_all(self.database.pool(), select_fields).await
    }

    pub async fn create_check(
        &self,
        select_fields: &[Field],
        account_uuid: &Uuid,
        project_uuid: &Uuid,
        name: &str,
    ) -> Result<Check> {
        let account_id = self.account.get_account_id(account_uuid).await?;
        let project_id = self.project.get_project_id(project_uuid).await?;

        let check = queries::insert(
            self.database.pool(),
            select_fields,
            account_id,
            project_id,
            name,
        )
        .await?;
        let uuid = check.uuid.as_ref().unwrap();

        tracing::trace!(
            account_uuid = account_uuid.to_string(),
            uuid = uuid.to_string(),
            name = name,
            "check created"
        );

        Ok(check)
    }

    pub async fn update_check(
        &self,
        uuid: &Uuid,
        select_fields: &[Field],
        update_fields: Vec<(Field, sea_query::Value)>,
    ) -> Result<(bool, Check)> {
        let (updated, check) =
            queries::update(self.database.pool(), uuid, select_fields, update_fields).await?;

        if updated {
            tracing::trace!(uuid = uuid.to_string(), "check updated");
        } else {
            tracing::trace!(uuid = uuid.to_string(), "no change, check not updated");
        }

        Ok((updated, check))
    }

    pub async fn delete_check(&self, uuid: &Uuid) -> Result<bool> {
        let deleted = queries::delete(self.database.pool(), uuid).await?;

        if deleted {
            tracing::trace!(uuid = uuid.to_string(), "check deleted");
        }

        Ok(deleted)
    }
}

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

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        if let Some(field) = NAME_TO_FIELD.get(value) {
            Ok(*field)
        } else {
            anyhow::bail!("unsupported Check variant '{}'", value);
        }
    }
}

mod queries {
    use super::*;

    pub async fn read_one(pool: &DbPool, select_fields: &[Field], uuid: &Uuid) -> Result<Check> {
        tracing::trace!(
            select = format!("{:?}", select_fields),
            uuid = uuid.to_string(),
            "reading check"
        );

        let (sql, params) = read_statement(select_fields)
            .and_where(Expr::col(Field::Uuid).eq(*uuid))
            .build(DbQueryBuilder::default());

        bind_query(sqlx::query(&sql), &params)
            .fetch_optional(pool)
            .await?
            .map(|row| from_row(&row, select_fields))
            .ok_or_else(|| RepositoryError::NotFound {
                entity_type: ENTITY_CHECK.to_string(),
                id: ShortId::from_uuid(uuid).to_string(),
            })?
    }

    pub async fn read_all(pool: &DbPool, select_fields: &[Field]) -> Result<Vec<Check>> {
        tracing::trace!(
            select = format!("{:?}", select_fields),
            "reading all checks"
        );

        let (sql, params) = read_statement(select_fields).build(DbQueryBuilder::default());

        bind_query(sqlx::query(&sql), &params)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|row| from_row(&row, select_fields))
            .collect()
    }

    pub async fn insert(
        pool: &DbPool,
        select_fields: &[Field],
        account_id: i64,
        project_id: i64,
        name: &str,
    ) -> Result<Check> {
        tracing::trace!(
            select = format!("{:?}", select_fields),
            account_id = account_id,
            project_id = project_id,
            name = name,
            "creating check"
        );

        let (sql, params) = insert_statement(select_fields, account_id, project_id, name)?
            .build(DbQueryBuilder::default());

        let row = bind_query(sqlx::query(&sql), &params)
            .fetch_one(pool)
            .await?;
        let issue = from_row(&row, select_fields)?;

        Ok(issue)
    }

    pub async fn update(
        pool: &DbPool,
        uuid: &Uuid,
        select_fields: &[Field],
        update_fields: Vec<(Field, sea_query::Value)>,
    ) -> Result<(bool, Check)> {
        let update_params: Vec<(Field, sea_query::Value)> = update_fields
            .into_iter()
            .filter(|i| Field::updatable().contains(&i.0))
            .collect();

        let query_builder = DbQueryBuilder::default();

        if tracing::event_enabled!(Level::TRACE) {
            let mut fields_to_update = String::from("[");
            for field in update_params.iter() {
                let _ = write!(
                    fields_to_update,
                    "{}={}",
                    field.0.as_ref(),
                    query_builder.value_to_string(&field.1)
                );
            }
            fields_to_update.push(']');
            tracing::trace!(
                uuid = uuid.to_string(),
                fields = fields_to_update,
                "updating check"
            );
        }

        let mut updated = false;
        if !update_params.is_empty() {
            let (sql, params) = update_statement(&update_params)
                .and_where(Expr::col(Field::Uuid).eq(*uuid))
                .and_where(Expr::col(Field::Deleted).eq(false))
                .build(query_builder);

            let rows_updated = bind_query(sqlx::query(&sql), &params)
                .execute(pool)
                .await?
                .rows_affected();

            updated = rows_updated > 0
        }

        let check = read_one(pool, select_fields, uuid).await?;
        Ok((updated, check))
    }

    pub async fn delete(pool: &DbPool, uuid: &Uuid) -> Result<bool> {
        tracing::trace!(uuid = uuid.to_string(), "deleting check");

        let (sql, params) = update_statement(&[
            (Field::Deleted, true.into()),
            (Field::DeletedAt, Utc::now().into()),
        ])
        .and_where(Expr::col(Field::Uuid).eq(*uuid))
        .build(DbQueryBuilder::default());

        let rows_deleted = bind_query(sqlx::query(&sql), &params)
            .execute(pool)
            .await?
            .rows_affected();

        Ok(rows_deleted > 0)
    }

    fn read_statement(selected_fields: &[Field]) -> SelectStatement {
        let mut statement = Query::select();

        statement
            .from(Field::Table)
            .columns(selected_fields.to_vec())
            .and_where(Expr::col(Field::Deleted).eq(false));

        statement
    }

    fn insert_statement(
        select_fields: &[Field],
        account_id: i64,
        project_id: i64,
        name: &str,
    ) -> Result<InsertStatement> {
        let mut statement = Query::insert();

        let now = Utc::now();
        let id = Uuid::new_v4();
        let short_id: ShortId = id.into();
        let ping_key = ShortId::new();

        statement
            .into_table(Field::Table)
            .columns([
                Field::AccountId,
                Field::ProjectId,
                Field::Uuid,
                Field::ShortId,
                Field::PingKey,
                Field::Name,
                Field::CreatedAt,
                Field::UpdatedAt,
            ])
            .values(vec![
                account_id.into(),
                project_id.into(),
                id.into(),
                short_id.into(),
                ping_key.into(),
                name.into(),
                now.into(),
                now.into(),
            ])?
            .returning(Query::returning().columns(select_fields.to_vec()));

        Ok(statement)
    }

    fn update_statement(values: &[(Field, sea_query::Value)]) -> UpdateStatement {
        let mut statement = Query::update();

        let mut values = values.to_vec();
        values.push((Field::UpdatedAt, Utc::now().into()));

        statement
            .table(Field::Table)
            .values(values)
            .and_where(Expr::col(Field::Deleted).eq(false));

        statement
    }

    fn from_row(row: &DbRow, select_fields: &[Field]) -> Result<Check> {
        let last_ping_at: Option<NaiveDateTime> =
            maybe_field_value(row, select_fields, &Field::LastPingAt)?;
        let created_at: Option<NaiveDateTime> =
            maybe_field_value(row, select_fields, &Field::CreatedAt)?;
        let updated_at: Option<NaiveDateTime> =
            maybe_field_value(row, select_fields, &Field::UpdatedAt)?;
        let uuid: Option<Uuid> = maybe_field_value(row, select_fields, &Field::Uuid)?;
        Ok(Check {
            uuid,
            ping_key: None,
            name: maybe_field_value(row, select_fields, &Field::Name)?,
            description: None,
            status: maybe_field_value(row, select_fields, &Field::Status)?,
            schedule_type: maybe_field_value(row, select_fields, &Field::ScheduleType)?,
            ping_period: maybe_field_value(row, select_fields, &Field::PingPeriod)?,
            ping_period_units: maybe_field_value(row, select_fields, &Field::PingPeriodUnits)?,
            grace_period: maybe_field_value(row, select_fields, &Field::GracePeriod)?,
            grace_period_units: maybe_field_value(row, select_fields, &Field::GracePeriodUnits)?,
            ping_cron_expression: maybe_field_value(
                row,
                select_fields,
                &Field::PingCronExpression,
            )?,
            last_ping_at: last_ping_at.map(|v| Utc.from_utc_datetime(&v)),
            created_at: created_at.map(|v| Utc.from_utc_datetime(&v)),
            updated_at: updated_at.map(|v| Utc.from_utc_datetime(&v)),
        })
    }
}
