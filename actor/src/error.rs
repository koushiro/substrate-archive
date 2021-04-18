#[derive(Debug, thiserror::Error)]
pub enum ActorError {
    #[error("{0}")]
    Api(#[from] sp_api::ApiError),

    #[error("{0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("{0}")]
    Disconnect(#[from] xtra::Disconnected),
    #[error("{0}")]
    Postgres(#[from] archive_postgres::SqlxError),
    #[error("{0}")]
    Kafka(#[from] archive_kafka::KafkaError),
}
