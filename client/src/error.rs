pub type BlockchainResult<T> = sp_blockchain::Result<T>;
pub type BlockchainError = sp_blockchain::Error;

pub fn backend_err<S: Into<String>>(err: S) -> BlockchainError {
    BlockchainError::Backend(err.into())
}

pub fn unknown_block_err<S: Into<String>>(err: S) -> BlockchainError {
    BlockchainError::UnknownBlock(err.into())
}
