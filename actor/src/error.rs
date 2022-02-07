pub use archive_kafka::KafkaError;
pub use archive_postgres::SqlxError;
pub use sp_blockchain::Error as BlockchainError;

#[derive(Debug, thiserror::Error)]
pub enum ActorError {
    #[error("{0}")]
    Blockchain(#[from] sp_blockchain::Error),

    #[error("{0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("{0}")]
    Disconnect(#[from] xtra::Disconnected),
    #[error("{0}")]
    Postgres(#[from] archive_postgres::SqlxError),
    #[error("{0}")]
    Kafka(#[from] archive_kafka::KafkaError),
}

impl From<sp_api::ApiError> for ActorError {
    fn from(err: sp_api::ApiError) -> Self {
        Self::Blockchain(sp_blockchain::Error::RuntimeApiError(err))
    }
}
