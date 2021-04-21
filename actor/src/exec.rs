use std::sync::Arc;

use sc_client_api::backend::{Backend, StateBackendFor};
use sp_api::{ApiExt, BlockId, Core as CoreApi, ProvideRuntimeApi};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use sp_storage::{StorageData, StorageKey};

use crate::error::BlockchainError;

/// In memory array of storage values.
pub type StorageCollection = Vec<(StorageKey, Option<StorageData>)>;
/// In memory arrays of storage values for multiple child tries.
pub type ChildStorageCollection = Vec<(StorageKey, StorageCollection)>;

#[derive(Default)]
pub struct StorageChanges {
    /// All changes to the main storage.
    ///
    /// A value of `None` means that it was deleted.
    pub main_storage_changes: StorageCollection,
    /// All changes to the child storages.
    pub child_storage_changes: ChildStorageCollection,
}

pub struct BlockExecutor<Block, B>
where
    Block: BlockT,
{
    backend: Arc<B>,
    id: BlockId<Block>,
    block: Block,
}

impl<Block, B> BlockExecutor<Block, B>
where
    Block: BlockT,
    B: Backend<Block> + ProvideRuntimeApi<Block>,
    <B as ProvideRuntimeApi<Block>>::Api:
        CoreApi<Block> + ApiExt<Block, StateBackend = StateBackendFor<B, Block>>,
{
    pub fn new(backend: Arc<B>, block: Block) -> Self {
        let parent_hash = block.header().parent_hash();
        let id = BlockId::Hash(*parent_hash);
        Self { backend, id, block }
    }

    pub fn into_storage_changes(self) -> Result<StorageChanges, BlockchainError> {
        let parent_hash = *self.block.header().parent_hash();
        let state = self.backend.state_at(self.id)?;
        let api = self.backend.runtime_api();
        api.execute_block(&self.id, self.block)?;
        let storage_changes = api
            .into_storage_changes(&state, None, parent_hash)
            .map_err(BlockchainError::StorageChanges)?;
        Ok(StorageChanges {
            main_storage_changes: storage_changes
                .main_storage_changes
                .into_iter()
                .map(|(key, value)| (StorageKey(key), value.map(StorageData)))
                .collect(),
            child_storage_changes: storage_changes
                .child_storage_changes
                .into_iter()
                .map(|(key, collection)| {
                    (
                        StorageKey(key),
                        collection
                            .into_iter()
                            .map(|(key, value)| (StorageKey(key), value.map(StorageData)))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect(),
        })
    }
}
