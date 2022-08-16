use std::time::Duration;

use miette::{Diagnostic, IntoDiagnostic, Result, WrapErr};
use sqlx::{migrate::Migrator, pool::PoolConnection, ConnectOptions};
use thiserror::Error;
use tracing::log::LevelFilter;

pub type DbType = sqlx::Postgres;
pub type DbConnectOptions = sqlx::postgres::PgConnectOptions;
pub type DbConnection = sqlx::postgres::PgConnection;
pub type DbPoolConnection = PoolConnection<DbType>;
pub type DbPoolOptions = sqlx::postgres::PgPoolOptions;
pub type DbTransaction<'t> = sqlx::Transaction<'t, DbType>;
pub type DbQueryBuilder = sea_query::PostgresQueryBuilder;

const SLOW_STATEMENT_THRESHOLD_MS: Duration = Duration::from_millis(5000);
static MIGRATOR: Migrator = sqlx::migrate!();

#[derive(Clone)]
pub struct Database {
    pool: sqlx::PgPool,
}

#[derive(Error, Diagnostic, Debug)]
pub enum DatabaseError {
    #[error("failed to parse URL '{0}'")]
    #[diagnostic(code(up::error::bad_argument))]
    MalformedUrl(String, #[source] sqlx::Error),
    #[error("SQL error: {0}")]
    #[diagnostic(code(up::error::sql))]
    GenericSqlError(#[from] sqlx::Error),
}

impl Database {
    async fn new(url: &str, min_connections: u32, max_connections: u32) -> Result<Self> {
        let mut connection_options: DbConnectOptions = url
            .parse()
            .map_err(|e| DatabaseError::MalformedUrl(url.to_string(), e))?;

        connection_options.log_statements(LevelFilter::Trace);
        connection_options.log_slow_statements(LevelFilter::Info, SLOW_STATEMENT_THRESHOLD_MS);

        let pool = DbPoolOptions::new()
            .min_connections(min_connections)
            .max_connections(max_connections)
            .connect_with(connection_options)
            .await
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to connect to database using URL '{}'", url))?;

        tracing::debug!(
            url = url,
            min_connections = min_connections,
            max_connections = max_connections,
            "connected to database"
        );
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<()> {
        for migration in MIGRATOR.migrations.iter() {
            tracing::debug!(
                desc = migration.description.to_string(),
                "migration {:0>3}",
                migration.version
            );
        }

        let result = MIGRATOR
            .run(&self.pool)
            .await
            .into_diagnostic()
            .wrap_err_with(|| "failed to perform database migration".to_string());

        if result.is_ok() {
            tracing::debug!(
                count = MIGRATOR.migrations.len(),
                "all migration(s) applied"
            )
        }

        result
    }

    pub async fn connection(&self) -> Result<DbPoolConnection, sqlx::Error> {
        self.pool.acquire().await
    }

    pub async fn transaction(&self) -> Result<DbTransaction, sqlx::Error> {
        self.pool.begin().await
    }
}

pub async fn connect(url: &str, min_connections: u32, max_connections: u32) -> Result<Database> {
    tracing::debug!(url = url, "connecting to database");
    Database::new(url, min_connections, max_connections).await
}
