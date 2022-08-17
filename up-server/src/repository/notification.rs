use chrono::NaiveDateTime;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    database::Database,
    notifier::Notifier,
    repository::{RepositoryError, Result},
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

    pub async fn read_one(&self, check_uuid: &Uuid, uuid: &Uuid) -> Result<Notification> {
        let mut conn = self.database.connection().await?;

        tracing::trace!(
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
                check_id = (SELECT id FROM checks WHERE uuid = $1)
                AND
                uuid = $2
                AND
                deleted = false
        ";

        let check: Option<Notification> = sqlx::query_as(sql)
            .bind(check_uuid)
            .bind(uuid)
            .fetch_optional(&mut *conn)
            .await?;

        check.ok_or_else(|| RepositoryError::NotFound {
            entity_type: ENTITY_NOTIFICATION.to_string(),
            id: ShortId::from_uuid(uuid).to_string(),
        })
    }

    pub async fn read_all(&self, check_uuid: &Uuid) -> Result<Vec<Notification>> {
        let mut conn = self.database.connection().await?;

        tracing::trace!("reading all notifications");

        let sql = r"
            SELECT
                *
            FROM
                notifications
            WHERE
                check_id = (SELECT id FROM checks WHERE uuid = $1)
                AND
                deleted = false
        ";

        let notifications: Vec<Notification> = sqlx::query_as(sql)
            .bind(check_uuid)
            .fetch_all(&mut *conn)
            .await?;

        Ok(notifications)
    }

    pub async fn create(
        &self,
        check_uuid: &Uuid,
        request: CreateNotification,
    ) -> Result<Notification> {
        let mut tx = self.database.transaction().await?;

        let sql = r"
            INSERT INTO notifications (
                check_id,
                uuid,
                shortid,
                name,
                notification_type,
                email,
                url,
                max_retries
            ) VALUES (
                (SELECT id FROM checks WHERE uuid = $1),
                $2,
                $3,
                $4,
                $5,
                $6,
                $7,
                COALESCE($8,5)
            ) RETURNING *
        ";

        let uuid = Uuid::new_v4();
        let short_id: ShortId = uuid.into();

        let notification: Notification = sqlx::query_as(sql)
            .bind(check_uuid)
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
        check_uuid: &Uuid,
        uuid: &Uuid,
        request: UpdateNotification,
    ) -> Result<Notification> {
        let mut tx = self.database.transaction().await?;

        let sql = r"
            UPDATE
                notifications
            SET
                name = COALESCE($3, name),
                email = COALESCE($4, email),
                url = COALESCE($5, url),
                max_retries = COALESCE($6, max_retries),
                updated_at = NOW() AT TIME ZONE 'UTC'
            WHERE
                check_id = (SELECT id FROM checks WHERE uuid = $1)
                AND
                uuid = $2
                AND
                deleted = false
            RETURNING *
        ";

        let notification: Option<Notification> = sqlx::query_as(sql)
            .bind(check_uuid)
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

    pub async fn delete(&self, check_uuid: &Uuid, uuid: &Uuid) -> Result<bool> {
        let mut tx = self.database.transaction().await?;

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
                check_id = (SELECT id FROM checks WHERE uuid = $1)
                AND
                uuid = $2
        ";

        let deleted = sqlx::query(sql)
            .bind(check_uuid)
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
}
