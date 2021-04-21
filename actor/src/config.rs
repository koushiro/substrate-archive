pub use archive_kafka::KafkaConfig;
pub use archive_postgres::PostgresConfig;

#[derive(Clone, Debug)]
pub struct ActorConfig {
    pub postgres: PostgresConfig,
    pub dispatcher: DispatcherConfig,
}

#[derive(Clone, Debug)]
pub struct DispatcherConfig {
    pub kafka: Option<KafkaConfig>,
    // others
}
