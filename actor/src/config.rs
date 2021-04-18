use archive_kafka::KafkaConfig;
use archive_postgres::PostgresConfig;

#[derive(Clone, Debug)]
pub struct ActorConfig {
    pub postgres: PostgresConfig,
    pub dispatcher: ActorDispatcherConfig,
}

#[derive(Clone, Debug)]
pub struct ActorDispatcherConfig {
    pub kafka: Option<KafkaConfig>,
    // others
}
