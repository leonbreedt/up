use chrono::NaiveDateTime;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::Identity,
    database::{Database, DbConnection},
    notifier::Notifier,
    repository::{check::ENTITY_CHECK, RepositoryError, Result},
    shortid::ShortId,
};

const ENTITY_NOTIFICATION: &str = "notification";

#[derive(sqlx::FromRow)]
pub struct Notification {
    pub id: i64,
    pub uuid: Uuid,
    pub name: String,
    pub notification_type: NotificationType,
    pub email: Option<String>,
    pub url: Option<String>,
    pub max_retries: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

pub struct CreateNotification {
    pub notification_type: NotificationType,
    pub name: Option<String>,
    pub email: Option<String>,
    pub url: Option<String>,
    pub max_retries: Option<i32>,
}

pub struct UpdateNotification {
    pub name: Option<String>,
    pub notification_type: Option<NotificationType>,
    pub email: Option<String>,
    pub url: Option<String>,
    pub max_retries: Option<i32>,
}

#[derive(sqlx::FromRow, Debug)]
pub struct NotificationAlert {
    pub id: i64,
    pub check_uuid: Uuid,
    pub notification_type: NotificationType,
    pub name: String,
    pub email: Option<String>,
    pub url: Option<String>,
    pub retries_remaining: i32,
    pub max_retries: i32,
    pub last_ping_at: Option<NaiveDateTime>,
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
            Self::Email => "EMAIL".to_string(),
            Self::Webhook => "WEBHOOK".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct NotificationRepository {
    database: Database,
}

impl NotificationRepository {
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn read_one(
        &self,
        identity: &Identity,
        project_uuid: &Uuid,
        check_uuid: &Uuid,
        uuid: &Uuid,
    ) -> Result<Notification> {
        identity.ensure_assigned_to_project(project_uuid)?;
        let project_id = identity.get_project_id(project_uuid)?;

        let mut conn = self.database.connection().await?;

        let (check_id, account_id) = self
            .get_check_account_id(&mut conn, check_uuid, project_id, &identity.account_ids())
            .await?;

        tracing::trace!(
            project_uuid = project_uuid.to_string(),
            check_uuid = check_uuid.to_string(),
            uuid = uuid.to_string(),
            "reading notification"
        );

        let sql = r"
            SELECT
                *
            FROM
                notifications
            WHERE
                uuid = $1
                AND
                check_id = $2
                AND
                account_id = $3
                AND
                project_id = $4
                AND
                deleted = false
        ";

        let check: Option<Notification> = sqlx::query_as(sql)
            .bind(uuid)
            .bind(check_id)
            .bind(account_id)
            .bind(project_id)
            .fetch_optional(&mut conn)
            .await?;

        check.ok_or_else(|| RepositoryError::NotFound {
            entity_type: ENTITY_NOTIFICATION.to_string(),
            id: ShortId::from_uuid(uuid).to_string(),
        })
    }

    pub async fn read_all(
        &self,
        identity: &Identity,
        project_uuid: &Uuid,
        check_uuid: &Uuid,
    ) -> Result<Vec<Notification>> {
        identity.ensure_assigned_to_project(project_uuid)?;
        let project_id = identity.get_project_id(project_uuid)?;

        let mut conn = self.database.connection().await?;

        tracing::trace!(
            project_uuid = project_uuid.to_string(),
            check_uuid = check_uuid.to_string(),
            "reading all notifications"
        );

        let sql = r"
            SELECT
                *
            FROM
                notifications
            WHERE
                check_id = (
                    SELECT
                        id
                    FROM
                        checks
                    WHERE
                        uuid = $1
                        AND
                        project_id = $2
                        AND
                        account_id = ANY($3)
                        AND
                        deleted = false
                )
                AND
                account_id = (
                    SELECT
                        account_id
                    FROM
                        checks
                    WHERE
                        uuid = $1
                        AND
                        project_id = $2
                        AND
                        account_id = ANY($3)
                        AND
                        deleted = false
                )
                AND
                project_id = $2
                AND
                deleted = false
        ";

        let notifications: Vec<Notification> = sqlx::query_as(sql)
            .bind(check_uuid)
            .bind(project_id)
            .bind(&identity.account_ids())
            .fetch_all(&mut conn)
            .await?;

        Ok(notifications)
    }

    pub async fn create(
        &self,
        identity: &Identity,
        project_uuid: &Uuid,
        check_uuid: &Uuid,
        request: CreateNotification,
    ) -> Result<Notification> {
        identity.ensure_assigned_to_project(project_uuid)?;
        let project_id = identity.get_project_id(project_uuid)?;

        let mut tx = self.database.transaction().await?;

        let (check_id, account_id) = self
            .get_check_account_id(&mut tx, check_uuid, project_id, &identity.account_ids())
            .await?;

        let sql = r"
            INSERT INTO notifications (
                check_id,
                account_id,
                project_id,
                uuid,
                shortid,
                name,
                notification_type,
                email,
                url,
                max_retries
            ) VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7,
                $8,
                $9,
                $10
            )
            RETURNING *
        ";

        let uuid = Uuid::new_v4();
        let short_id: ShortId = uuid.into();

        let notification: Notification = sqlx::query_as(sql)
            .bind(check_id)
            .bind(account_id)
            .bind(project_id)
            .bind(uuid)
            .bind(short_id.to_string())
            .bind(&request.name)
            .bind(&request.notification_type)
            .bind(&request.email)
            .bind(&request.url)
            .bind(&request.max_retries)
            .fetch_one(&mut tx)
            .await?;

        tx.commit().await?;

        tracing::trace!(
            check_uuid = check_uuid.to_string(),
            uuid = notification.uuid.to_string(),
            name = request.name,
            "notification created"
        );

        Ok(notification)
    }

    pub async fn update(
        &self,
        identity: &Identity,
        project_uuid: &Uuid,
        check_uuid: &Uuid,
        uuid: &Uuid,
        request: UpdateNotification,
    ) -> Result<Notification> {
        identity.ensure_assigned_to_project(project_uuid)?;
        let project_id = identity.get_project_id(project_uuid)?;

        let mut tx = self.database.transaction().await?;

        let (check_id, account_id) = self
            .get_check_account_id(&mut tx, check_uuid, project_id, &identity.account_ids())
            .await?;

        let sql = r"
            UPDATE
                notifications
            SET
                name = COALESCE($5, name),
                email = COALESCE($6, email),
                url = COALESCE($7, url),
                max_retries = COALESCE($8, max_retries),
                updated_at = NOW() AT TIME ZONE 'UTC'
            WHERE
                check_id = $1
                AND
                account_id = $2
                AND
                project_id = $3
                AND
                uuid = $4
                AND
                deleted = false
            RETURNING *
        ";

        let notification: Option<Notification> = sqlx::query_as(sql)
            .bind(check_id)
            .bind(account_id)
            .bind(project_id)
            .bind(uuid)
            .bind(&request.name)
            .bind(&request.email)
            .bind(&request.url)
            .bind(&request.max_retries)
            .fetch_optional(&mut tx)
            .await?;

        if notification.is_none() {
            return Err(RepositoryError::NotFound {
                entity_type: ENTITY_NOTIFICATION.to_string(),
                id: ShortId::from_uuid(uuid).to_string(),
            });
        }

        tx.commit().await?;

        tracing::trace!(
            check_uuid = check_uuid.to_string(),
            uuid = uuid.to_string(),
            "notification updated"
        );

        Ok(notification.unwrap())
    }

    pub async fn delete(
        &self,
        identity: &Identity,
        project_uuid: &Uuid,
        check_uuid: &Uuid,
        uuid: &Uuid,
    ) -> Result<bool> {
        identity.ensure_assigned_to_project(project_uuid)?;
        let project_id = identity.get_project_id(project_uuid)?;

        let mut tx = self.database.transaction().await?;

        let (check_id, account_id) = self
            .get_check_account_id(&mut tx, check_uuid, project_id, &identity.account_ids())
            .await?;

        tracing::trace!(
            check_uuid = check_uuid.to_string(),
            uuid = uuid.to_string(),
            "deleting notification"
        );

        let sql = r"
            UPDATE
                notifications
            SET
                deleted = true,
                deleted_at = NOW() AT TIME ZONE 'UTC'
            WHERE
                check_id = $1
                AND
                account_id = $2
                AND
                project_id = $3
                AND
                uuid = $4
        ";

        let deleted = sqlx::query(sql)
            .bind(check_id)
            .bind(account_id)
            .bind(project_id)
            .bind(uuid)
            .execute(&mut tx)
            .await?
            .rows_affected()
            > 0;

        tx.commit().await?;

        if deleted {
            tracing::trace!(uuid = uuid.to_string(), "notification deleted");
        }

        Ok(deleted)
    }

    pub async fn send_alert_batch(&self, notifier: &Notifier) -> Result<Vec<NotificationAlert>> {
        let mut tx = self.database.transaction().await?;

        let sql = r"
            SELECT
                a.id,
                a.retries_remaining,
                n.notification_type,
                n.email,
                n.url,
                n.max_retries,
                c.uuid as check_uuid,
                (CASE LTRIM(RTRIM(n.name))
                WHEN '' THEN c.name
                ELSE n.name
                END) AS name,
                c.last_ping_at
            FROM
                notification_alerts a
                INNER JOIN
                notifications n ON n.id = a.notification_id AND n.deleted = false
                INNER JOIN
                checks c ON c.id = n.check_id AND c.deleted = false
            WHERE
                delivery_status = 'QUEUED'
                OR
                (delivery_status = 'FAILED' AND retries_remaining > 0)
            ORDER BY
                a.created_at ASC
            LIMIT 10
            FOR UPDATE SKIP LOCKED
            ";

        let alerts: Vec<NotificationAlert> = sqlx::query_as(sql).fetch_all(&mut tx).await?;
        let mut sent_alerts = Vec::new();
        let mut failed_alerts = Vec::new();

        for alert in alerts {
            match notifier.send_alert(&alert).await {
                Ok(_) => sent_alerts.push(alert),
                Err(e) => {
                    tracing::error!("failed to send alert: {:?}", e);
                    failed_alerts.push(alert)
                }
            }
        }

        for alert in sent_alerts.iter() {
            // TODO: Include confirmation from server, e.g. Message ID or HTTP status?
            let sql = r"
            UPDATE notification_alerts
            SET delivery_status = 'DELIVERED', finished_at = NOW() AT TIME ZONE 'UTC'
            WHERE id = $1
            ";

            let result = sqlx::query(sql).bind(alert.id).execute(&mut tx).await?;
            if result.rows_affected() != 1 {
                tracing::warn!(
                    alert_id = alert.id,
                    "alert delivered successfully, but failed to update status, duplicate will be sent later",
                );
            } else {
                tracing::debug!(alert_id = alert.id, "alert delivered successfully");
            }
        }

        for alert in failed_alerts {
            // TODO: Include confirmation from server, e.g. Message ID or HTTP status?
            let sql = if alert.retries_remaining <= 0 {
                r"
                    UPDATE notification_alerts
                    SET
                        delivery_status = 'FAILED',
                        retries_remaining = 0,
                        finished_at = NOW() AT TIME ZONE 'UTC'
                    WHERE
                        id = $1
                    RETURNING
                        retries_remaining
                "
            } else {
                r"
                    UPDATE notification_alerts
                    SET
                        delivery_status = 'FAILED',
                        retries_remaining = retries_remaining - 1,
                        finished_at = NOW() AT TIME ZONE 'UTC'
                    WHERE
                        id = $1
                    RETURNING
                        retries_remaining
                "
            };

            let row = sqlx::query(sql).bind(alert.id).fetch_one(&mut tx).await?;
            let retries_remaining: i32 = row.get("retries_remaining");

            if retries_remaining > 0 {
                tracing::debug!(
                    retries_remaining = retries_remaining,
                    alert_id = alert.id,
                    "will retry sending alert"
                );
            } else {
                tracing::debug!(
                    alert_id = alert.id,
                    "exceeded max_retries, giving up sending alert"
                );
            }
        }

        tx.commit().await?;

        Ok(sent_alerts)
    }

    async fn get_check_account_id(
        &self,
        conn: &mut DbConnection,
        check_uuid: &Uuid,
        project_id: i64,
        account_ids: &[i64],
    ) -> Result<(i64, i64)> {
        let sql = r"
            SELECT
                id,
                account_id
            FROM
                checks
            WHERE
                uuid = $1
                AND
                project_id = $2
                AND
                account_id = ANY($3)
                AND
                deleted = false
            LIMIT 1
        ";

        let ids: Option<(i64, i64)> = sqlx::query_as(sql)
            .bind(check_uuid)
            .bind(project_id)
            .bind(account_ids)
            .fetch_optional(conn)
            .await?;

        ids.ok_or(RepositoryError::NotFound {
            entity_type: ENTITY_CHECK.to_string(),
            id: ShortId::from(check_uuid).to_string(),
        })
    }
}
