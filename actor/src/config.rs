use serde::{Deserialize, Serialize};

use archive_kafka::KafkaConfig;
use archive_postgres::PostgresConfig;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActorConfig {
    pub postgres: PostgresConfig,
    // dispatcher
    pub kafka: Option<KafkaConfig>,
}
