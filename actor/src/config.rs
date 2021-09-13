use serde::{Deserialize, Serialize};

use sp_storage::Storage;

pub use archive_kafka::KafkaConfig;
pub use archive_postgres::PostgresConfig;

#[derive(Clone, Debug)]
pub struct ActorConfig {
    pub postgres: PostgresConfig,
    pub dispatcher: Option<DispatcherConfig>,
    pub genesis: Storage,
    pub scheduler: SchedulerConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub start_block: Option<u32>,
    pub max_block_load: u32,
    pub interval_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DispatcherConfig {
    pub kafka: Option<KafkaConfig>,
    // others
}
