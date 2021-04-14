use std::sync::Arc;

use sc_client_api::execution_extensions::{ExecutionExtensions, ExecutionStrategies};
use sc_executor::{NativeExecutionDispatch, NativeExecutor};
use sp_core::testing::TaskExecutor;
use sp_runtime::traits::Block as BlockT;

use crate::error::ArchiveClientResult;
use crate::{
    backend::{BackendConfig, ReadOnlyBackend},
    client::Client,
    config::ClientConfig,
    database::{RocksDbConfig, SecondaryRocksDb},
};

/// Archive client type.
pub type ArchiveClient<Block, Executor, RA> =
    Client<ArchiveBackend<Block>, ArchiveCallExecutor<Block, Executor>, Block, RA>;

/// Archive client backend type.
pub type ArchiveBackend<Block> = ReadOnlyBackend<Block>;

/// Archive client call executor type.
pub type ArchiveCallExecutor<Block, Executor> =
    sc_service::LocalCallExecutor<ReadOnlyBackend<Block>, NativeExecutor<Executor>>;

pub fn new_archive_client<Block, Executor, RA>(
    config: ClientConfig,
) -> ArchiveClientResult<ArchiveClient<Block, Executor, RA>>
where
    Block: BlockT,
    Executor: NativeExecutionDispatch + 'static,
{
    let backend = new_secondary_rocksdb_backend(config.rocksdb, Default::default())?;

    let executor = ArchiveCallExecutor::new(
        backend.clone(),
        NativeExecutor::<Executor>::new(
            config.wasm_exec_method.into(),
            config.default_heap_pages,
            config.max_runtime_instances,
        ),
        Box::new(TaskExecutor::new()),
        sc_service::ClientConfig {
            offchain_worker_enabled: config.offchain_worker.enabled,
            offchain_indexing_api: config.offchain_worker.indexing_enabled,
            wasm_runtime_overrides: config.wasm_runtime_overrides,
        },
    )?;

    let execution_extensions = ExecutionExtensions::new(ExecutionStrategies::default(), None, None);

    Ok(ArchiveClient::new(backend, executor, execution_extensions))
}

fn new_secondary_rocksdb_backend<Block>(
    rocksdb: RocksDbConfig,
    backend: BackendConfig,
) -> ArchiveClientResult<Arc<ReadOnlyBackend<Block>>>
where
    Block: BlockT,
{
    let db = Arc::new(SecondaryRocksDb::open(rocksdb)?);
    let backend = Arc::new(ReadOnlyBackend::new(db, backend)?);
    Ok(backend)
}
