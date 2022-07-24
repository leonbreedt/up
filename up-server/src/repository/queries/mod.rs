#![allow(clippy::manual_map)]
sea_query::sea_query_driver_sqlite!();

pub use sea_query_driver_sqlite::bind_query;

pub mod account;
pub mod check;

use sqlx::{Row, ValueRef};

use super::{dto::ModelField, Result};
use crate::database::{DbRow, DbType};

pub fn maybe_field_value<'r, F, V>(row: &'r DbRow, selection: &[F], field: &F) -> Result<Option<V>>
where
    F: ModelField,
    V: sqlx::Decode<'r, DbType> + sqlx::Type<DbType>,
{
    if selection.contains(field) {
        let index = field.as_ref();
        let value_ref = row.try_get_raw(index)?;
        if value_ref.is_null() {
            Ok(None)
        } else {
            Ok(Some(row.try_get(index)?))
        }
    } else {
        Ok(None)
    }
}
