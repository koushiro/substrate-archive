use std::sync::Arc;

use sc_client_api::execution_extensions::{ExecutionExtensions, ExecutionStrategies};
use sc_executor::{NativeExecutionDispatch, NativeExecutor};
use sp_core::testing::TaskExecutor;
use sp_runtime::traits::Block as BlockT;
use sp_state_machine::ExecutionStrategy;

use crate::{
    backend::ReadOnlyBackend,
    client::Client,
    config::ClientConfig,
    database::{RocksDbConfig, SecondaryRocksDb},
    error::{BlockchainError, BlockchainResult},
};

/// Archive client backend type.
pub type ArchiveBackend<Block> = ReadOnlyBackend<Block>;

/// Archive client call executor type.
pub type ArchiveCallExecutor<Block, Executor> =
    sc_service::LocalCallExecutor<Block, ReadOnlyBackend<Block>, NativeExecutor<Executor>>;

/// Archive client type.
pub type ArchiveClient<Block, Executor, RA> =
    Client<Block, ArchiveBackend<Block>, ArchiveCallExecutor<Block, Executor>, RA>;

pub fn new_secondary_rocksdb_backend<Block>(
    rocksdb: RocksDbConfig,
) -> BlockchainResult<ArchiveBackend<Block>>
where
    Block: BlockT,
{
    let db =
        SecondaryRocksDb::open(rocksdb).map_err(|err| BlockchainError::Backend(err.to_string()))?;
    let db = Arc::new(db);
    let backend = ReadOnlyBackend::new(db, Default::default())?;
    Ok(backend)
}

pub fn new_archive_client<Block, Executor, RA>(
    backend: Arc<ArchiveBackend<Block>>,
    config: ClientConfig,
) -> BlockchainResult<ArchiveClient<Block, Executor, RA>>
where
    Block: BlockT,
    Executor: NativeExecutionDispatch + 'static,
{
    let executor = ArchiveCallExecutor::new(
        backend.clone(),
        NativeExecutor::<Executor>::new(
            config.executor.wasm_exec_method.into(),
            config.executor.default_heap_pages,
            config.executor.max_runtime_instances,
        ),
        Box::new(TaskExecutor::new()),
        sc_service::ClientConfig {
            offchain_worker_enabled: config.offchain_worker.enabled,
            offchain_indexing_api: config.offchain_worker.indexing_enabled,
            wasm_runtime_overrides: config.wasm_runtime_overrides,
            wasm_runtime_substitutes: Default::default(),
        },
    )?;

    let execution_extensions = ExecutionExtensions::new(
        ExecutionStrategies {
            syncing: ExecutionStrategy::AlwaysWasm,
            importing: ExecutionStrategy::AlwaysWasm,
            block_construction: ExecutionStrategy::AlwaysWasm,
            offchain_worker: ExecutionStrategy::AlwaysWasm,
            other: ExecutionStrategy::AlwaysWasm,
        },
        None,
        None,
    );

    Ok(ArchiveClient::new(backend, executor, execution_extensions))
}
