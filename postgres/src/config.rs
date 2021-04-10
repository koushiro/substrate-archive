use std::time::Duration;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct PostgresConfig {
    // connection options (parse for the PgConnectOptions)
    pub uri: String,
    // connection pool options
    pub min_connections: u32,
    pub max_connections: u32,
    pub connect_timeout: Duration,
    pub idle_timeout: Option<Duration>,
    pub max_lifetime: Option<Duration>,
}

impl PostgresConfig {
    pub fn uri(&self) -> &str {
        &self.uri
    }
}
