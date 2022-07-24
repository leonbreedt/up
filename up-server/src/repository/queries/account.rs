use sea_query::{Expr, Query, SelectStatement};
use sqlx::Row;

use crate::{
    database::{DbPool, DbQueryBuilder},
    repository::{dto::account::Field, RepositoryError, Result},
};

use super::bind_query;

pub async fn get_account_id(pool: &DbPool, uuid: &str) -> Result<i64> {
    let (sql, params) = read_statement(&[Field::Id])
        .and_where(Expr::col(Field::Uuid).eq(uuid))
        .build(DbQueryBuilder::default());
    let row = bind_query(sqlx::query(&sql), &params)
        .fetch_optional(pool)
        .await?;
    if let Some(row) = row {
        Ok(row.try_get("id")?)
    } else {
        return Err(RepositoryError::InvalidArgument(
            "account_id".to_string(),
            format!("{} does not exist", uuid),
        ));
    }
}

fn read_statement(selected_fields: &[Field]) -> SelectStatement {
    let mut statement = Query::select();

    statement
        .from(Field::Table)
        .columns(selected_fields.to_vec())
        .and_where(Expr::col(Field::Deleted).eq(false));

    statement
}
