#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("{0}")]
    Actor(#[from] archive_actor::ActorError),

    #[error("{0}")]
    Client(#[from] sp_blockchain::Error),

    #[error("{0}")]
    Migration(#[from] archive_postgres::SqlxError),

    #[error("{0}")]
    FlumeSend(#[from] flume::SendError<()>),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Toml(#[from] toml::de::Error),
}
