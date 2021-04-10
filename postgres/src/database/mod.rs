pub mod insert;
pub mod query;

use sqlx::{
    error::Error as SqlxError,
    pool::PoolConnection,
    postgres::{PgPool, PgPoolOptions, Postgres},
};

use crate::config::PostgresConfig;

#[derive(Clone)]
pub struct PostgresDB {
    config: PostgresConfig,
    pool: PgPool,
}

impl PostgresDB {
    pub async fn new(config: PostgresConfig) -> Result<Self, SqlxError> {
        let pool = PgPoolOptions::new()
            .min_connections(config.min_connections)
            .max_connections(config.max_connections)
            .connect_timeout(config.connect_timeout)
            .idle_timeout(config.idle_timeout)
            .max_lifetime(config.max_lifetime)
            .connect(config.uri())
            .await?;
        Ok(Self { pool, config })
    }

    pub async fn conn(&self) -> Result<PoolConnection<Postgres>, SqlxError> {
        self.pool.acquire().await.map_err(Into::into)
    }

    pub fn config(&self) -> &PostgresConfig {
        &self.config
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}
