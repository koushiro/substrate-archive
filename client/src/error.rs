pub type BlockchainResult<T> = sp_blockchain::Result<T>;
pub type BlockchainError = sp_blockchain::Error;

pub fn backend_err<S: Into<String>>(err: S) -> BlockchainError {
    BlockchainError::Backend(err.into())
}

pub fn unknown_block_err<S: Into<String>>(err: S) -> BlockchainError {
    BlockchainError::UnknownBlock(err.into())
}

pub type ArchiveClientResult<T, E = ArchiveClientError> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum ArchiveClientError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Blockchain(#[from] Box<BlockchainError>),
}

impl From<BlockchainError> for ArchiveClientError {
    fn from(err: BlockchainError) -> Self {
        Self::Blockchain(Box::new(err))
    }
}
