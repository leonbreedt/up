use std::{collections::HashMap, fmt::Write as _, str::FromStr};

use chrono::{NaiveDateTime, Utc};
use lazy_static::lazy_static;
use sea_query::{Alias, Expr, Iden, Query, QueryBuilder, SimpleExpr};
use tracing::Level;
use uuid::Uuid;

use super::{bind_query, bind_query_as, ModelField};

use crate::repository::column_value;
use crate::{
    database::{Database, DbConnection, DbQueryBuilder},
    repository::{
        account::AccountRepository, project::ProjectRepository, QueryValue, RepositoryError, Result,
    },
    shortid::ShortId,
};

const ENTITY_NOTIFICATION: &str = "notification";

#[derive(Clone)]
pub struct NotificationRepository {
    database: Database,
    account: AccountRepository,
    project: ProjectRepository,
}

impl NotificationRepository {}

impl NotificationRepository {
    pub fn new(database: Database, account: AccountRepository, project: ProjectRepository) -> Self {
        Self {
            database,
            account,
            project,
        }
    }

    pub async fn read_one(&self, uuid: &Uuid) -> Result<Notification> {
        let mut conn = self.database.connection().await?;

        tracing::trace!(uuid = uuid.to_string(), "reading notification");

        self.read_one_internal(&mut conn, uuid).await
    }

    pub async fn read_all(&self) -> Result<Vec<Notification>> {
        let mut conn = self.database.connection().await?;

        tracing::trace!("reading all notifications");

        let (sql, params) = Query::select()
            .from(Field::Table)
            .columns(Field::all().to_vec())
            .and_where(Expr::col(Field::Deleted).eq(false))
            .build(DbQueryBuilder::default());

        let notifications = bind_query_as(sqlx::query_as(&sql), &params)
            .fetch_all(&mut *conn)
            .await?;

        Ok(notifications)
    }

    pub async fn create(
        &self,
        account_uuid: &Uuid,
        project_uuid: &Uuid,
        values: Vec<QueryValue<Field>>,
    ) -> Result<Notification> {
        let mut tx = self.database.transaction().await?;

        let account_id = self.account.get_account_id(&mut tx, account_uuid).await?;
        let project_id = self.project.get_project_id(&mut tx, project_uuid).await?;

        let mut name = String::new();

        if tracing::event_enabled!(Level::TRACE) {
            for value in &values {
                if value.field() == &Field::Name {
                    name = value.to_string();
                }
            }

            tracing::trace!(
                account_id = account_id,
                project_id = project_id,
                name = name,
                "creating notification"
            );
        }

        let mut values = values.clone();
        values.insert(0, column_value(Field::AccountId, account_id));
        values.insert(1, column_value(Field::ProjectId, project_id));

        let columns: Vec<Field> = values.iter().map(|v| *v.field()).collect();
        let exprs: Vec<SimpleExpr> = values.iter().map(|v| v.as_expr()).collect();

        let (sql, params) = Query::insert()
            .into_table(Field::Table)
            .columns(columns)
            .exprs(exprs)?
            .returning(Query::returning().columns(Field::all().to_vec()))
            .build(DbQueryBuilder::default());

        let notification: Notification = bind_query_as(sqlx::query_as(&sql), &params)
            .fetch_one(&mut tx)
            .await?;

        tx.commit().await?;

        tracing::trace!(
            account_uuid = account_uuid.to_string(),
            uuid = notification.uuid.to_string(),
            name = name,
            "notification created"
        );

        Ok(notification)
    }

    pub async fn update(
        &self,
        uuid: &Uuid,
        values: Vec<QueryValue<Field>>,
    ) -> Result<(bool, Notification)> {
        let mut tx = self.database.transaction().await?;

        let values: Vec<(Field, SimpleExpr)> = values
            .into_iter()
            .filter(|v| Field::updatable().contains(v.field()))
            .map(|v| {
                (
                    *v.field(),
                    match v {
                        QueryValue::Value(_, v) => SimpleExpr::Value(v),
                        QueryValue::Expression(_, e) => e,
                    },
                )
            })
            .collect();

        let query_builder = DbQueryBuilder::default();

        if tracing::event_enabled!(Level::TRACE) {
            let mut values_to_update = String::from("[");
            let mut first = true;
            for (field, value) in values.iter() {
                if !first {
                    let _ = write!(values_to_update, ", ");
                }
                first = false;
                let value_str = match value {
                    SimpleExpr::Value(v) => query_builder.value_to_string(v),
                    SimpleExpr::AsEnum(_, expr) => match expr.as_ref() {
                        SimpleExpr::Value(v) => query_builder.value_to_string(v),
                        _ => format!("{:?}", expr),
                    },
                    _ => format!("{:?}", value),
                };
                let _ = write!(values_to_update, "{}={}", field.as_ref(), value_str);
            }
            values_to_update.push(']');
            tracing::trace!(
                uuid = uuid.to_string(),
                values = values_to_update,
                "updating notification"
            );
        }

        let mut updated = false;
        if !values.is_empty() {
            let mut values = values.clone();

            let now_value: sea_query::Value = Utc::now().into();
            values.push((Field::UpdatedAt, SimpleExpr::Value(now_value)));

            let mut query = Query::update();
            let query = query.table(Field::Table);
            for (field, value) in values {
                query.value_expr(field, value);
            }

            let (sql, params) = query
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

        let notification = self.read_one_internal(&mut tx, uuid).await?;

        tx.commit().await?;

        if updated {
            tracing::trace!(uuid = uuid.to_string(), "notification updated");
        } else {
            tracing::trace!(
                uuid = uuid.to_string(),
                "no change, notification not updated"
            );
        }

        Ok((updated, notification))
    }

    pub async fn delete(&self, uuid: &Uuid) -> Result<bool> {
        let mut tx = self.database.transaction().await?;

        tracing::trace!(uuid = uuid.to_string(), "deleting notification");

        let (sql, params) = Query::update()
            .table(Field::Table)
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
            tracing::trace!(uuid = uuid.to_string(), "notification deleted");
        }

        Ok(deleted)
    }

    async fn read_one_internal(
        &self,
        conn: &mut DbConnection,
        uuid: &Uuid,
    ) -> Result<Notification> {
        let (sql, params) = Query::select()
            .from(Field::Table)
            .columns(Field::all().to_vec())
            .and_where(Expr::col(Field::Deleted).eq(false))
            .and_where(Expr::col(Field::Uuid).eq(*uuid))
            .build(DbQueryBuilder::default());

        let check: Option<Notification> = bind_query_as(sqlx::query_as(&sql), &params)
            .fetch_optional(&mut *conn)
            .await?;

        check.ok_or_else(|| RepositoryError::NotFound {
            entity_type: ENTITY_NOTIFICATION.to_string(),
            id: ShortId::from_uuid(uuid).to_string(),
        })
    }
}

#[derive(sqlx::Type)]
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

impl From<NotificationType> for SimpleExpr {
    fn from(t: NotificationType) -> Self {
        Expr::val(t.to_string()).as_enum(Alias::new("notification_type"))
    }
}

#[derive(sqlx::FromRow)]
pub struct Notification {
    pub uuid: Uuid,
    pub name: String,
    pub notification_type: NotificationType,
    pub email: Option<String>,
    pub url: Option<String>,
    pub max_retries: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Field {
    Table,
    Id,
    AccountId,
    ProjectId,
    Uuid,
    Name,
    NotificationType,
    Email,
    Url,
    MaxRetries,
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
            Field::NotificationType,
            Field::Email,
            Field::Url,
            Field::MaxRetries,
        ]
    }
}

lazy_static! {
    static ref NAME_TO_FIELD: HashMap<String, Field> = vec![
        (Field::Id.to_string(), Field::Id),
        (Field::AccountId.to_string(), Field::AccountId),
        (Field::ProjectId.to_string(), Field::ProjectId),
        (Field::Uuid.to_string(), Field::Uuid),
        (Field::Name.to_string(), Field::Name),
        (Field::NotificationType.to_string(), Field::NotificationType),
        (Field::Email.to_string(), Field::Email),
        (Field::Url.to_string(), Field::Url),
        (Field::MaxRetries.to_string(), Field::MaxRetries),
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
            Self::Table => "notifications",
            Self::Id => "id",
            Self::AccountId => "account_id",
            Self::ProjectId => "project_id",
            Self::Uuid => "uuid",
            Self::Name => "name",
            Self::NotificationType => "notification_type",
            Self::Email => "email",
            Self::Url => "url",
            Self::MaxRetries => "max_retries",
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
            anyhow::bail!("unsupported Notification variant '{}'", value);
        }
    }
}
