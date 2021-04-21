mod backend;
mod builder;
mod client;
mod config;
mod database;
mod error;
mod utils;

pub use self::{
    backend::{BackendConfig, ReadOnlyBackend},
    builder::{new_archive_client, ArchiveBackend, ArchiveCallExecutor, ArchiveClient},
    client::Client,
    config::ClientConfig,
    database::{RocksDbConfig, SecondaryRocksDb},
    error::{BlockchainError, BlockchainResult},
};

#[allow(unused)]
pub(crate) mod columns {
    pub const META: u32 = crate::utils::COLUMN_META;
    pub const STATE: u32 = 1;
    pub const STATE_META: u32 = 2;
    /// maps hashes to lookup keys and numbers to canon hashes.
    pub const KEY_LOOKUP: u32 = 3;
    pub const HEADER: u32 = 4;
    pub const BODY: u32 = 5;
    pub const JUSTIFICATIONS: u32 = 6;
    pub const CHANGES_TRIE: u32 = 7;
    pub const AUX: u32 = 8;
    /// Offchain workers local storage
    pub const OFFCHAIN: u32 = 9;
    pub const CACHE: u32 = 10;
    /// Transactions
    pub const TRANSACTION: u32 = 11;
}
