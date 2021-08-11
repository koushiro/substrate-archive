use sp_storage::Storage;

pub use archive_kafka::KafkaConfig;
pub use archive_postgres::PostgresConfig;

#[derive(Clone, Debug)]
pub struct ActorConfig {
    pub genesis: Storage,
    pub postgres: PostgresConfig,
    pub dispatcher: Option<DispatcherConfig>,
    pub max_block_load: u32,
    pub interval_ms: u64,
}

#[derive(Clone, Debug)]
pub struct DispatcherConfig {
    pub kafka: Option<KafkaConfig>,
    // others
}
