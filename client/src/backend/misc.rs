use std::collections::HashMap;

use sc_client_api::backend::{BlockImportOperation, NewBlockState, TransactionForSB};
use sp_blockchain::well_known_cache_keys;
use sp_core::offchain::OffchainStorage;
use sp_runtime::{generic::BlockId, traits::Block as BlockT, Justification, Justifications};
use sp_state_machine::{
    ChildStorageCollection, IndexOperation, OffchainChangesCollection, StorageCollection,
};
use sp_storage::{StateVersion, Storage};

use crate::{backend::state::RefTrackingState, error::BlockchainResult};

pub struct BlockImportOperationImpl;
impl<Block> BlockImportOperation<Block> for BlockImportOperationImpl
where
    Block: BlockT,
{
    type State = RefTrackingState<Block>;

    fn state(&self) -> BlockchainResult<Option<&Self::State>> {
        log::warn!("Block import operations not supported with a read-only backend");
        Ok(None)
    }

    fn set_block_data(
        &mut self,
        _header: Block::Header,
        _body: Option<Vec<Block::Extrinsic>>,
        _indexed_body: Option<Vec<Vec<u8>>>,
        _justifications: Option<Justifications>,
        _state: NewBlockState,
    ) -> BlockchainResult<()> {
        log::warn!("Block state may not be set with a read-only backend");
        Ok(())
    }

    fn update_cache(&mut self, _cache: HashMap<well_known_cache_keys::Id, Vec<u8>>) {
        log::warn!("No cache on a read-only backend");
    }

    fn update_db_storage(
        &mut self,
        _update: TransactionForSB<Self::State, Block>,
    ) -> BlockchainResult<()> {
        log::warn!("Cannot modify storage of a read-only backend, storage not updated");
        Ok(())
    }

    fn set_genesis_state(
        &mut self,
        _storage: Storage,
        _commit: bool,
        _state_version: StateVersion,
    ) -> BlockchainResult<Block::Hash> {
        log::warn!("Cannot set genesis state of a read-only backend, storage not updated");
        Ok(Default::default())
    }

    fn reset_storage(
        &mut self,
        _storage: Storage,
        _state_version: StateVersion,
    ) -> BlockchainResult<Block::Hash> {
        log::warn!("Cannot modify storage of a read-only backend, storage not reset");
        Ok(Default::default())
    }

    fn update_storage(
        &mut self,
        _update: StorageCollection,
        _child_update: ChildStorageCollection,
    ) -> BlockchainResult<()> {
        log::warn!("Cannot modify storage of a read-only backend, child storage not updated");
        Ok(())
    }

    fn update_offchain_storage(
        &mut self,
        _offchain_update: OffchainChangesCollection,
    ) -> BlockchainResult<()> {
        log::warn!("Cannot modify storage of a read-only backend, offchain storage not updated");
        Ok(())
    }

    fn insert_aux<I>(&mut self, _ops: I) -> BlockchainResult<()>
    where
        I: IntoIterator<Item = (Vec<u8>, Option<Vec<u8>>)>,
    {
        log::warn!("Cannot modify storage of a read-only backend, aux not inserted");
        Ok(())
    }

    fn mark_finalized(
        &mut self,
        _id: BlockId<Block>,
        _justification: Option<Justification>,
    ) -> BlockchainResult<()> {
        log::warn!("Cannot modify storage of a read-only backend, finalized not marked");
        Ok(())
    }

    fn mark_head(&mut self, _id: BlockId<Block>) -> BlockchainResult<()> {
        log::warn!("Cannot modify storage of a read-only backend, head not marked");
        Ok(())
    }

    fn update_transaction_index(&mut self, _index: Vec<IndexOperation>) -> BlockchainResult<()> {
        log::warn!("Cannot modify storage of a read-only backend, transaction index not updated");
        Ok(())
    }
}

#[derive(Clone)]
pub struct OffchainStorageImpl;
impl OffchainStorage for OffchainStorageImpl {
    fn set(&mut self, _prefix: &[u8], _key: &[u8], _value: &[u8]) {}

    fn remove(&mut self, _prefix: &[u8], _key: &[u8]) {}

    fn get(&self, _prefix: &[u8], _key: &[u8]) -> Option<Vec<u8>> {
        None
    }

    fn compare_and_set(
        &mut self,
        _prefix: &[u8],
        _key: &[u8],
        _old_value: Option<&[u8]>,
        _new_value: &[u8],
    ) -> bool {
        false
    }
}
