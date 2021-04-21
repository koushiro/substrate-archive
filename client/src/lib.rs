mod backend;
mod builder;
mod client;
mod config;
mod database;
mod error;
mod utils;

pub use self::{
    backend::{BackendConfig, ReadOnlyBackend},
    builder::{
        new_archive_client, new_secondary_rocksdb_backend, ArchiveBackend, ArchiveCallExecutor,
        ArchiveClient,
    },
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

use sc_client_api::backend::{Backend as BackendT, StateBackendFor};
use sp_api::{CallApiAt, ConstructRuntimeApi, ProvideRuntimeApi};
use sp_runtime::traits::Block as BlockT;

/// super trait for accessing methods that rely on internal runtime api
pub trait ApiAccess<Block, Backend, RA>:
    ProvideRuntimeApi<Block, Api = RA::RuntimeApi>
    + CallApiAt<Block, StateBackend = StateBackendFor<Backend, Block>>
    + Send
    + Sync
    + Sized
where
    Block: BlockT,
    Backend: BackendT<Block>,
    RA: ConstructRuntimeApi<Block, Self>,
{
}

impl<Client, Block, Backend, RA> ApiAccess<Block, Backend, RA> for Client
where
    Block: BlockT,
    Backend: BackendT<Block>,
    RA: ConstructRuntimeApi<Block, Self>,
    Client: ProvideRuntimeApi<Block, Api = RA::RuntimeApi>
        + CallApiAt<Block, StateBackend = StateBackendFor<Backend, Block>>
        + Send
        + Sync
        + Sized,
{
}
