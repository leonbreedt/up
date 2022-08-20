use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::{
    auth::Identity,
    database::Database,
    repository::{RepositoryError, Result},
    shortid::ShortId,
};

pub const ENTITY_CHECK: &str = "check";

#[derive(sqlx::FromRow)]
pub struct Check {
    pub id: i64,
    pub uuid: Uuid,
    pub account_id: i64,
    pub project_id: i64,
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

pub struct CreateCheck {
    pub account_uuid: Uuid,
    pub project_uuid: Uuid,
    pub name: String,
}

pub struct UpdateCheck {
    pub name: Option<String>,
}

#[derive(Clone)]
pub struct CheckRepository {
    database: Database,
}

impl CheckRepository {
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn read_one(
        &self,
        identity: &Identity,
        project_uuid: &Uuid,
        check_uuid: &Uuid,
    ) -> Result<Check> {
        identity.ensure_assigned_to_project(project_uuid)?;
        let project_id = identity.get_project_id(project_uuid)?;

        let mut conn = self.database.connection().await?;

        tracing::trace!(
            project_uuid = project_uuid.to_string(),
            check_uuid = check_uuid.to_string(),
            "reading check"
        );

        let sql = r"
            SELECT
                *
            FROM
                checks
            WHERE
                project_id = $1
                AND
                uuid = $2
                AND
                account_id = ANY($3)
                AND
                deleted = false
        ";

        sqlx::query_as(sql)
            .bind(project_id)
            .bind(check_uuid)
            .bind(&identity.project_ids())
            .fetch_optional(&mut conn)
            .await?
            .ok_or_else(|| RepositoryError::NotFound {
                entity_type: ENTITY_CHECK.to_string(),
                id: ShortId::from_uuid(check_uuid).to_string(),
            })
    }

    pub async fn read_all(&self, identity: &Identity, project_uuid: &Uuid) -> Result<Vec<Check>> {
        identity.ensure_assigned_to_project(project_uuid)?;
        let project_id = identity.get_project_id(project_uuid)?;

        let mut conn = self.database.connection().await?;

        tracing::trace!(project_uuid = project_uuid.to_string(), "reading checks");

        let sql = r"
            SELECT
                *
            FROM
                checks
            WHERE
                project_id = $1
                AND
                account_id = ANY($2)
                AND
                deleted = false
        ";

        Ok(sqlx::query_as(sql)
            .bind(project_id)
            .bind(&identity.account_ids())
            .fetch_all(&mut conn)
            .await?)
    }

    pub async fn create(
        &self,
        identity: &Identity,
        project_uuid: &Uuid,
        request: CreateCheck,
    ) -> Result<Check> {
        identity.ensure_assigned_to_project(project_uuid)?;
        identity.ensure_assigned_to_account(&request.account_uuid)?;

        let project_id = identity.get_project_id(project_uuid)?;
        let account_id = identity.get_account_id(&request.account_uuid)?;

        let mut tx = self.database.transaction().await?;

        tracing::trace!(
            account_uuid = request.account_uuid.to_string(),
            project_uuid = request.project_uuid.to_string(),
            name = request.name,
            "creating check"
        );

        let uuid = Uuid::new_v4();
        let short_id: ShortId = uuid.into();
        let ping_key = ShortId::new();

        let sql = r"
            INSERT INTO checks (
                account_id,
                project_id,
                uuid,
                shortid,
                ping_key,
                name
            ) VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6
            ) RETURNING *
        ";

        let check: Check = sqlx::query_as(sql)
            .bind(account_id)
            .bind(project_id)
            .bind(uuid)
            .bind(short_id.to_string())
            .bind(ping_key.to_string())
            .bind(&request.name)
            .fetch_one(&mut tx)
            .await?;

        tx.commit().await?;

        tracing::trace!(
            account_uuid = &request.account_uuid.to_string(),
            uuid = check.uuid.to_string(),
            name = &request.name,
            "check created"
        );

        Ok(check)
    }

    pub async fn update(
        &self,
        identity: &Identity,
        project_uuid: &Uuid,
        uuid: &Uuid,
        request: UpdateCheck,
    ) -> Result<Check> {
        identity.ensure_assigned_to_project(project_uuid)?;
        let project_id = identity.get_project_id(project_uuid)?;

        let mut tx = self.database.transaction().await?;

        let sql = r"
            UPDATE
                checks
            SET
                name = COALESCE($4,name)
            WHERE
                uuid = $1
                AND
                project_id = $2
                AND
                account_id = ANY($3)
                AND
                deleted = false
            RETURNING *
        ";

        let check: Option<Check> = sqlx::query_as(sql)
            .bind(uuid)
            .bind(project_id)
            .bind(&identity.account_ids())
            .bind(&request.name)
            .fetch_optional(&mut tx)
            .await?;

        if check.is_none() {
            return Err(RepositoryError::NotFound {
                entity_type: ENTITY_CHECK.to_string(),
                id: ShortId::from_uuid(uuid).to_string(),
            });
        }

        tx.commit().await?;

        tracing::trace!(uuid = uuid.to_string(), "check updated");

        Ok(check.unwrap())
    }

    pub async fn delete(
        &self,
        identity: &Identity,
        project_uuid: &Uuid,
        uuid: &Uuid,
    ) -> Result<bool> {
        identity.ensure_assigned_to_project(project_uuid)?;
        let project_id = identity.get_project_id(project_uuid)?;

        let mut tx = self.database.transaction().await?;

        tracing::trace!(uuid = uuid.to_string(), "deleting check");

        let sql = r"
            UPDATE
                checks
            SET
                deleted = true,
                deleted_at = NOW() AT TIME ZONE 'UTC'
            WHERE
                uuid = $1
                AND
                project_id = $2
                AND
                account_id = ANY($3)
        ";

        let deleted = sqlx::query(sql)
            .bind(uuid)
            .bind(project_id)
            .bind(&identity.account_ids())
            .execute(&mut tx)
            .await?
            .rows_affected()
            > 0;

        tx.commit().await?;

        if deleted {
            tracing::trace!(uuid = uuid.to_string(), "check deleted");
        }

        Ok(deleted)
    }

    pub async fn ping(&self, key: &str) -> Result<Option<Uuid>> {
        let mut tx = self.database.transaction().await?;

        let sql = r"
            UPDATE
                checks
            SET
                status = 'UP',
                last_ping_at = NOW() AT TIME ZONE 'UTC'
            WHERE
                ping_key = $1
                AND
                deleted = false
            RETURNING
                uuid
        ";

        let check_uuid: Option<(Uuid,)> = sqlx::query_as(sql)
            .bind(key)
            .fetch_optional(&mut tx)
            .await?;

        tx.commit().await?;

        Ok(check_uuid.map(|id| id.0))
    }

    /// [`enqueue_alerts_for_overdue_pings`] not called by APIs, so no access checks needed.
    pub async fn enqueue_alerts_for_overdue_pings(&self) -> Result<()> {
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
                o.last_ping_at
            FROM (
                SELECT
                  c.*,
                  (NOW() AT TIME ZONE 'UTC' > last_ping_at + c.ping_period_interval) AS ping_overdue,
                  (NOW() AT TIME ZONE 'UTC' > last_ping_at + c.ping_period_interval + c.grace_period_interval) AS late_ping_overdue
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

        let overdue_pings: Vec<(i64, Uuid, CheckStatus, String, NaiveDateTime)> =
            sqlx::query_as(overdue_ping_sql).fetch_all(&mut tx).await?;

        for ping_details in overdue_pings {
            let (check_id, check_uuid, check_status, check_name, last_ping_at) = ping_details;

            let sql = r"
                UPDATE
                    checks
                SET
                    status = 'DOWN'
                WHERE
                    uuid = $1
                    AND
                    deleted = false
            ";

            let rows_updated = sqlx::query(sql)
                .bind(check_uuid)
                .execute(&mut tx)
                .await?
                .rows_affected();

            if rows_updated == 0 {
                tracing::error!(
                    check_uuid = check_uuid.to_string(),
                    "failed to set status of check to DOWN, no rows updated"
                );
                return Err(RepositoryError::NotFound {
                    entity_type: ENTITY_CHECK.to_string(),
                    id: ShortId::from_uuid(&check_uuid).to_string(),
                });
            }

            let sql = r"
                SELECT
                    id,
                    notification_type,
                    email,
                    url,
                    max_retries
                FROM
                    notifications
                WHERE
                    check_id = $1
                    AND NOT EXISTS (
                        SELECT 1
                        FROM notification_alerts a
                        WHERE
                            a.notification_id = notifications.id
                    )
            ";

            #[allow(clippy::type_complexity)]
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
                    notification_id,
                    check_status,
                    retries_remaining
                ) VALUES (
                    $1,
                    $2,
                    $3
                );
                ";
                sqlx::query(sql)
                    .bind(notification_id)
                    .bind(check_status)
                    .bind(retries_remaining)
                    .execute(&mut tx)
                    .await?;

                tracing::debug!(
                    check_uuid = check_uuid.to_string(),
                    name = check_name,
                    alert_type = notification_type.to_string(),
                    email = email,
                    url = url,
                    last_ping_at = last_ping_at.to_string(),
                    "enqueuing alert"
                );
            }
        }

        tx.commit().await?;

        Ok(())
    }
}
