use std::{collections::HashMap, fmt::Debug, fmt::Write as _, hash::Hash, str::FromStr};

use chrono::{NaiveDateTime, Utc};
use lazy_static::lazy_static;
use sea_query::{Alias, Expr, Iden, Query, QueryBuilder};
use sqlx::Row;
use tracing::Level;
use uuid::Uuid;

use super::{bind_query, bind_query_as, ModelField};
use crate::{
    database::{Database, DbConnection, DbQueryBuilder, DbRow},
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

    pub async fn read_one(&self, uuid: &Uuid) -> Result<Check> {
        let mut conn = self.database.connection().await?;

        tracing::trace!(uuid = uuid.to_string(), "reading check");

        self.read_one_internal(&mut conn, uuid).await
    }

    pub async fn read_all(&self) -> Result<Vec<Check>> {
        let mut conn = self.database.connection().await?;

        tracing::trace!("reading all checks");

        let (sql, params) = Query::select()
            .from(Field::Table)
            .columns(Field::all().to_vec())
            .and_where(Expr::col(Field::Deleted).eq(false))
            .build(DbQueryBuilder::default());

        let checks = bind_query_as(sqlx::query_as(&sql), &params)
            .fetch_all(&mut *conn)
            .await?;

        Ok(checks)
    }

    pub async fn enqueue_notification_alerts_for_overdue_pings(&self) -> Result<()> {
        let mut tx = self.database.transaction().await?;

        tracing::trace!("checking for overdue pings");

        // Overdue pings on checks:
        //
        // - Are for checks that have been pinged successfully at least once
        // - Are not currently paused
        // - Have not been pinged before ping period elapsed
        // - Have not been pinged before late ping grace period elapsed

        let overdue_ping_sql = r#"
            SELECT
                o.id,
                o.uuid,
                o.status,
                o.name,
                o.last_ping_at,
                o.ping_overdue,
                o.ping_overdue_at,
                o.late_ping_overdue,
                o.late_ping_overdue_at
            FROM (
                SELECT
                  c.*,
                  (NOW() AT TIME ZONE 'UTC' > last_ping_at + c.ping_period_interval) AS ping_overdue,
                  (last_ping_at + c.ping_period_interval) AS ping_overdue_at,
                  (NOW() AT TIME ZONE 'UTC' > last_ping_at + c.ping_period_interval + c.grace_period_interval) AS late_ping_overdue,
                  (last_ping_at + c.ping_period_interval + c.grace_period_interval) AS late_ping_overdue_at
                FROM (
                       SELECT
                           id,
                           uuid,
                           name,
                           status,
                           last_ping_at,
                           (CASE ping_period_units
                                WHEN 'HOURS' THEN INTERVAL '1' HOUR
                                WHEN 'DAYS' THEN INTERVAL '1' DAY
                                END * ping_period) AS ping_period_interval,
                           (CASE grace_period_units
                                WHEN 'HOURS' THEN INTERVAL '1' HOUR
                                WHEN 'DAYS' THEN INTERVAL '1' DAY
                                END * grace_period) AS grace_period_interval
                       FROM
                           checks
                       WHERE
                               deleted = false
                         AND last_ping_at IS NOT NULL
                         AND status NOT IN ('CREATED', 'PAUSED')
                   ) AS c
                ) AS o
            WHERE
                o.ping_overdue = true
                OR
                o.late_ping_overdue = true;
        "#;

        let overdue_pings: Vec<(
            i64,
            Uuid,
            CheckStatus,
            String,
            NaiveDateTime,
            bool,
            Option<NaiveDateTime>,
            bool,
            Option<NaiveDateTime>,
        )> = sqlx::query_as(overdue_ping_sql).fetch_all(&mut tx).await?;

        for ping_details in overdue_pings {
            let (
                check_id,
                check_uuid,
                check_status,
                check_name,
                last_ping_at,
                _ping_overdue,
                _ping_overdue_at,
                _late_ping_overdue,
                _late_ping_overdue_at,
            ) = ping_details;

            let sql = r"
                SELECT
                    n.id,
                    n.notification_type,
                    n.email,
                    n.url,
                    n.max_retries
                FROM
                    check_notifications cn INNER JOIN notifications n ON cn.notification_id = n.id AND cn.check_id = $1
                WHERE
                    NOT EXISTS (
                        SELECT 1
                        FROM notification_alerts a
                        WHERE
                            a.check_id = cn.check_id
                            AND
                            a.notification_id = n.id
                    )
            ";

            let notifications_to_alert: Vec<(
                i64,
                NotificationType,
                Option<String>,
                Option<String>,
                i32,
            )> = sqlx::query_as(sql)
                .bind(check_id)
                .fetch_all(&mut tx)
                .await?;

            for (notification_id, notification_type, email, url, retries_remaining) in
                notifications_to_alert
            {
                let sql = r"
                INSERT INTO notification_alerts (
                    check_id,
                    check_status,
                    notification_id,
                    retries_remaining
                ) VALUES (
                    $1,
                    $2,
                    $3,
                    $4
                );
                ";
                sqlx::query(sql)
                    .bind(check_id)
                    .bind(check_status.clone())
                    .bind(notification_id)
                    .bind(retries_remaining)
                    .execute(&mut tx)
                    .await?;

                tracing::debug!(
                    check_id = ShortId::from(check_uuid).to_string(),
                    name = check_name,
                    alert_type = notification_type.to_string(),
                    email = email,
                    url = url,
                    last_ping_at = last_ping_at.to_string(),
                    "sending alert"
                );
            }
        }

        tx.commit().await?;

        Ok(())
    }

    pub async fn create(
        &self,
        account_uuid: &Uuid,
        project_uuid: &Uuid,
        name: &str,
    ) -> Result<Check> {
        let mut tx = self.database.transaction().await?;

        let account_id = self.account.get_account_id(&mut tx, account_uuid).await?;
        let project_id = self.project.get_project_id(&mut tx, project_uuid).await?;

        tracing::trace!(
            account_id = account_id,
            project_id = project_id,
            name = name,
            "creating check"
        );

        let now = Utc::now();
        let id = Uuid::new_v4();
        let short_id: ShortId = id.into();
        let ping_key = ShortId::new();

        let (sql, params) = Query::insert()
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
            .returning(Query::returning().columns(Field::all().to_vec()))
            .build(DbQueryBuilder::default());

        let check: Check = bind_query_as(sqlx::query_as(&sql), &params)
            .fetch_one(&mut tx)
            .await?;

        tx.commit().await?;

        tracing::trace!(
            account_uuid = account_uuid.to_string(),
            uuid = check.uuid.to_string(),
            name = name,
            "check created"
        );

        Ok(check)
    }

    pub async fn update(
        &self,
        uuid: &Uuid,
        update_fields: Vec<(Field, sea_query::Value)>,
    ) -> Result<(bool, Check)> {
        let mut tx = self.database.transaction().await?;

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
            let mut values = update_params.clone();

            values.push((Field::UpdatedAt, Utc::now().into()));

            let (sql, params) = Query::update()
                .table(Field::Table)
                .values(values)
                .and_where(Expr::col(Field::Deleted).eq(false))
                .and_where(Expr::col(Field::Uuid).eq(*uuid))
                .and_where(Expr::col(Field::Deleted).eq(false))
                .build(query_builder);

            let rows_updated = bind_query(sqlx::query(&sql), &params)
                .execute(&mut tx)
                .await?
                .rows_affected();

            updated = rows_updated > 0
        }

        let check = self.read_one_internal(&mut tx, uuid).await?;

        tx.commit().await?;

        if updated {
            tracing::trace!(uuid = uuid.to_string(), "check updated");
        } else {
            tracing::trace!(uuid = uuid.to_string(), "no change, check not updated");
        }

        Ok((updated, check))
    }

    pub async fn delete(&self, uuid: &Uuid) -> Result<bool> {
        let mut tx = self.database.transaction().await?;

        tracing::trace!(uuid = uuid.to_string(), "deleting check");

        let (sql, params) = Query::update()
            .values(vec![
                (Field::Deleted, true.into()),
                (Field::DeletedAt, Utc::now().into()),
            ])
            .and_where(Expr::col(Field::Uuid).eq(*uuid))
            .build(DbQueryBuilder::default());

        let rows_deleted = bind_query(sqlx::query(&sql), &params)
            .execute(&mut tx)
            .await?
            .rows_affected();

        let deleted = rows_deleted > 0;

        tx.commit().await?;

        if deleted {
            tracing::trace!(uuid = uuid.to_string(), "check deleted");
        }

        Ok(deleted)
    }

    pub async fn ping(&self, key: &str) -> Result<Option<Uuid>> {
        let mut tx = self.database.transaction().await?;

        let (sql, params) = Query::select()
            .from(Field::Table)
            .columns(vec![Field::Id, Field::Uuid])
            .and_where(Expr::col(Field::PingKey).eq(key))
            .and_where(Expr::col(Field::Deleted).eq(false))
            .build(DbQueryBuilder::default());

        let result: (i64, Uuid) = bind_query_as(sqlx::query_as(&sql), &params)
            .fetch_optional(&mut tx)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundPingKey {
                key: key.to_string(),
            })?;

        let (sql, params) = Query::update()
            .table(Field::Table)
            .value(Field::LastPingAt, Utc::now().into())
            .value_expr(
                Field::Status,
                Expr::val(CheckStatus::Up.to_string()).as_enum(Alias::new("check_status")),
            )
            .and_where(Expr::col(Field::Id).eq(result.0))
            .and_where(Expr::col(Field::Deleted).eq(false))
            .build(DbQueryBuilder::default());

        let rows_updated = bind_query(sqlx::query(&sql), &params)
            .execute(&mut tx)
            .await?
            .rows_affected();

        tx.commit().await?;

        if rows_updated > 0 {
            Ok(Some(result.1))
        } else {
            Ok(None)
        }
    }

    async fn read_one_internal(&self, conn: &mut DbConnection, uuid: &Uuid) -> Result<Check> {
        tracing::trace!(uuid = uuid.to_string(), "reading check");

        let (sql, params) = Query::select()
            .from(Field::Table)
            .columns(Field::all().to_vec())
            .and_where(Expr::col(Field::Deleted).eq(false))
            .and_where(Expr::col(Field::Uuid).eq(*uuid))
            .build(DbQueryBuilder::default());

        let check: Option<Check> = bind_query_as(sqlx::query_as(&sql), &params)
            .fetch_optional(&mut *conn)
            .await?;

        check.ok_or_else(|| RepositoryError::NotFound {
            entity_type: ENTITY_CHECK.to_string(),
            id: ShortId::from_uuid(uuid).to_string(),
        })
    }
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "schedule_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScheduleType {
    Simple,
    Cron,
}

#[derive(sqlx::Type, Copy, Clone)]
#[sqlx(type_name = "check_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CheckStatus {
    Up,
    Down,
    Created,
}

impl ToString for CheckStatus {
    fn to_string(&self) -> String {
        match self {
            CheckStatus::Up => "UP".to_string(),
            CheckStatus::Down => "DOWN".to_string(),
            CheckStatus::Created => "CREATED".to_string(),
        }
    }
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "period_units", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PeriodUnits {
    Hours,
    Minutes,
    Days,
}

#[derive(sqlx::FromRow)]
pub struct Check {
    pub uuid: Uuid,
    pub ping_key: String,
    pub name: String,
    pub description: String,
    pub status: CheckStatus,
    pub schedule_type: ScheduleType,
    pub ping_period: i32,
    pub ping_period_units: PeriodUnits,
    pub ping_cron_expression: Option<String>,
    pub grace_period: i32,
    pub grace_period_units: PeriodUnits,
    pub last_ping_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

// TODO: Add Notification

#[derive(sqlx::Type, Debug)]
#[sqlx(type_name = "notification_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NotificationType {
    Email,
    Webhook,
}

impl ToString for NotificationType {
    fn to_string(&self) -> String {
        match self {
            NotificationType::Email => "EMAIL".to_string(),
            NotificationType::Webhook => "WEBHOOK".to_string(),
        }
    }
}

pub struct OverdueCheck {
    pub inner: Check,
    // TODO: Temporary, we should load a notification entry from an integrations table
    pub email: String,
}

impl TryFrom<DbRow> for OverdueCheck {
    type Error = RepositoryError;

    fn try_from(row: DbRow) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            inner: Check {
                uuid: row.try_get(Field::Uuid.as_ref())?,
                ping_key: row.try_get(Field::PingKey.as_ref())?,
                name: row.try_get(Field::Name.as_ref())?,
                description: row.try_get(Field::Description.as_ref())?,
                status: row.try_get(Field::Status.as_ref())?,
                schedule_type: row.try_get(Field::ScheduleType.as_ref())?,
                ping_period: row.try_get(Field::PingPeriod.as_ref())?,
                ping_period_units: row.try_get(Field::PingPeriodUnits.as_ref())?,
                ping_cron_expression: row.try_get(Field::PingCronExpression.as_ref())?,
                grace_period: row.try_get(Field::GracePeriod.as_ref())?,
                grace_period_units: row.try_get(Field::GracePeriodUnits.as_ref())?,
                last_ping_at: row.try_get(Field::LastPingAt.as_ref())?,
                created_at: row.try_get(Field::CreatedAt.as_ref())?,
                updated_at: row.try_get(Field::UpdatedAt.as_ref())?,
            },
            email: row.try_get("email")?,
        })
    }
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
