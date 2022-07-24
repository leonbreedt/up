use std::time::Duration;

use anyhow::Result;
use sqlx::{migrate::Migrator, ConnectOptions};
use tracing::log::LevelFilter;

pub type DbConnectOptions = sqlx::sqlite::SqliteConnectOptions;
pub type DbPool = sqlx::sqlite::SqlitePool;
pub type DbPoolOptions = sqlx::sqlite::SqlitePoolOptions;
pub type DbQueryBuilder = sea_query::SqliteQueryBuilder;
pub type DbRow = sqlx::sqlite::SqliteRow;
pub type DbType = sqlx::Sqlite;

const SLOW_STATEMENT_THRESHOLD_MS: Duration = Duration::from_millis(100);
static MIGRATOR: Migrator = sqlx::migrate!();

#[derive(Clone)]
pub struct Database {
    pool: DbPool,
}

impl Database {
    async fn new(url: &str, min_connections: u32, max_connections: u32) -> Result<Self> {
        let mut connection_options: DbConnectOptions = url.parse()?;
        connection_options.log_statements(LevelFilter::Trace);
        connection_options.log_slow_statements(LevelFilter::Info, SLOW_STATEMENT_THRESHOLD_MS);
        let pool = DbPoolOptions::new()
            .min_connections(min_connections)
            .max_connections(max_connections)
            .connect_with(connection_options.create_if_missing(true))
            .await?;
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
            .map_err(|e| anyhow::anyhow!(e));

        if result.is_ok() {
            tracing::debug!(
                count = MIGRATOR.migrations.len(),
                "all migration(s) applied"
            )
        }

        result
    }

    pub fn pool(&self) -> &DbPool {
        &self.pool
    }
}

pub async fn connect(url: &str, min_connections: u32, max_connections: u32) -> Result<Database> {
    tracing::debug!(url = url, "connecting to database");
    Database::new(url, min_connections, max_connections)
        .await
        .map_err(|e| anyhow::anyhow!(e))
}
