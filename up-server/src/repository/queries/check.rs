use std::fmt::Write as _;

use chrono::{NaiveDateTime, TimeZone, Utc};
use sea_query::{Expr, InsertStatement, Query, QueryBuilder, SelectStatement, UpdateStatement};
use tracing::Level;
use uuid::Uuid;

use super::{bind_query, maybe_field_value};
use crate::{
    database::{DbPool, DbQueryBuilder, DbRow},
    repository::{
        dto::check::{Check, Field},
        queries::{account::get_account_id, project::get_project_id},
        RepositoryError, Result,
    },
    shortid::ShortId,
};

const ENTITY_CHECK: &str = "check";

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
    account_uuid: &Uuid,
    project_uuid: &Uuid,
    name: &str,
) -> Result<Check> {
    tracing::trace!(
        select = format!("{:?}", select_fields),
        account_uuid = account_uuid.to_string(),
        name = name,
        "creating check"
    );

    let account_id = get_account_id(pool, account_uuid).await?;
    let project_id = get_project_id(pool, project_uuid).await?;

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
        ping_cron_expression: maybe_field_value(row, select_fields, &Field::PingCronExpression)?,
        last_ping_at: last_ping_at.map(|v| Utc.from_utc_datetime(&v)),
        created_at: created_at.map(|v| Utc.from_utc_datetime(&v)),
        updated_at: updated_at.map(|v| Utc.from_utc_datetime(&v)),
    })
}
