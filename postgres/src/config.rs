use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostgresConfig {
    // connection options (parse for the PgConnectOptions)
    pub uri: String,
    // connection pool options
    pub min_connections: u32,
    pub max_connections: u32,
    pub connect_timeout: u64,      // seconds
    pub idle_timeout: Option<u64>, // seconds
    pub max_lifetime: Option<u64>, // seconds
    // Entirely disables statement logging (both slow and regular).
    pub disable_statement_logging: bool,
}

impl PostgresConfig {
    pub fn uri(&self) -> &str {
        &self.uri
    }
}
