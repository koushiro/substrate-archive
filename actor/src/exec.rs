use std::sync::Arc;

use sc_client_api::backend::{Backend, StateBackendFor};
use sp_api::{ApiExt, ApiRef, BlockId, Core as CoreApi};
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

pub struct BlockExecutor<'a, Block, B, Api>
where
    Block: BlockT,
{
    block: Block,
    backend: &'a Arc<B>,
    api: ApiRef<'a, Api>,
}

impl<'a, Block, B, Api> BlockExecutor<'a, Block, B, Api>
where
    Block: BlockT,
    B: Backend<Block>,
    Api: CoreApi<Block> + ApiExt<Block, StateBackend = StateBackendFor<B, Block>>,
{
    pub fn new(block: Block, backend: &'a Arc<B>, api: ApiRef<'a, Api>) -> Self {
        Self {
            block,
            backend,
            api,
        }
    }

    pub fn into_storage_changes(self) -> Result<StorageChanges, BlockchainError> {
        let parent_hash = *self.block.header().parent_hash();
        let parent_block_id = BlockId::Hash(parent_hash);
        let state = self.backend.state_at(parent_block_id)?;

        // FIXME: ????
        // Wasm runtime calculates a different number of digest items
        // than what we have in the block
        // We don't do anything with consensus
        // so digest isn't very important (we don't currently index digest items anyway)
        // popping a digest item has no effect on storage changes afaik
        let (mut header, ext) = self.block.deconstruct();
        // log::info!(
        //     "Execute block #{} into storage, id = {:?}, hash={:?}, parent_hash={:?}, digest={:?}, state_root={:?}, extrinsic_root={:?}",
        //     header.number(), parent_block_id, header.hash(), parent_hash, header.digest(), header.state_root(), header.extrinsics_root()
        // );
        header.digest_mut().pop();
        let block = Block::new(header, ext);

        self.api.execute_block(&parent_block_id, block)?;
        let storage_changes = self
            .api
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