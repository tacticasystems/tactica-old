//! PostgreSQL adapters for Tactica modules.
#![deny(missing_docs)]

use std::str::FromStr;

use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};

/// PostgreSQL connection configuration.
pub mod config;
/// Identity and session repository implementations.
pub mod identity;

/// A postgres database connection pool.
#[derive(Debug, Clone)]
pub struct Postgres {
    pub(crate) pool: PgPool,
}

impl Postgres {
    /// Connects a PostgreSQL pool from configuration.
    pub async fn new(config: config::DatabaseConfig) -> Result<Self, anyhow::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .idle_timeout(config.idle_timeout)
            .min_connections(config.min_connections)
            .connect_with(PgConnectOptions::from_str(&config.url)?)
            .await?;

        Ok(Self { pool })
    }

    /// Applies all pending embedded migrations.
    pub async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
        sqlx::migrate!().run(&self.pool).await
    }
}
