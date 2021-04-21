pub mod blockchain;
pub mod misc;
pub mod state;

use std::{collections::HashSet, sync::Arc};

use sc_client_api::{
    backend::{AuxStore, Backend, PrunableStateChangesTrieStorage},
    client::BlockBackend,
    UsageInfo,
};
use sc_client_db::{DbHash, DbState, TransactionStorageMode};
use sc_state_db::{PruningMode, StateDb};
use sp_blockchain::{Backend as BlockchainBackend, HeaderBackend, HeaderMetadata};
use sp_consensus::BlockStatus;
use sp_database::Database;
use sp_runtime::{
    generic::{BlockId, SignedBlock},
    traits::{Block as BlockT, NumberFor, SaturatedConversion},
    Justification, Justifications,
};
use sp_state_machine::Storage;

use self::{
    blockchain::BlockchainDb,
    state::{DbGenesisStorage, RefTrackingState, StateMetaDb, StorageDb},
};
use crate::{
    columns,
    error::{backend_err, unknown_block_err, BlockchainError, BlockchainResult},
};

/// Backend configurations.
pub struct BackendConfig {
    /// State pruning mode.
    pub state_pruning: PruningMode,
    /// Block body/Transaction storage scheme.
    pub transaction_storage: TransactionStorageMode,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            state_pruning: PruningMode::ArchiveCanonical,
            transaction_storage: TransactionStorageMode::BlockBody,
        }
    }
}

pub struct ReadOnlyBackend<Block: BlockT> {
    storage: Arc<StorageDb<Block>>,
    blockchain: BlockchainDb<Block>,
    // changes_tries_storage: DbChangesTrieStorage<Block>,
    config: BackendConfig,
}

impl<Block: BlockT> ReadOnlyBackend<Block> {
    /// Create a new instance of database backend.
    ///
    /// The pruning window is how old a block must be before the state is pruned.
    pub fn new(db: Arc<dyn Database<DbHash>>, config: BackendConfig) -> BlockchainResult<Self> {
        let state_db = StateDb::new(config.state_pruning.clone(), true, &StateMetaDb(&*db))
            .map_err(BlockchainError::from_state_db)?;
        let storage_db = StorageDb {
            db: db.clone(),
            state_db,
            prefix_keys: true,
        };
        let blockchain = BlockchainDb::new(db.clone(), config.transaction_storage)?;

        Ok(ReadOnlyBackend {
            storage: Arc::new(storage_db),
            blockchain,
            config,
        })
    }
}

impl<Block> Backend<Block> for ReadOnlyBackend<Block>
where
    Block: BlockT,
{
    type BlockImportOperation = self::misc::BlockImportOperationImpl;
    type Blockchain = self::blockchain::BlockchainDb<Block>;
    type State = self::state::RefTrackingState<Block>;
    type OffchainStorage = self::misc::OffchainStorageImpl;

    fn begin_operation(&self) -> BlockchainResult<Self::BlockImportOperation> {
        log::warn!("Block import operations are not supported for read-only backend");
        Ok(self::misc::BlockImportOperationImpl)
    }

    fn begin_state_operation(
        &self,
        _operation: &mut Self::BlockImportOperation,
        _block: BlockId<Block>,
    ) -> BlockchainResult<()> {
        log::warn!("State operations not supported, operation not begun");
        Ok(())
    }

    fn commit_operation(&self, _transaction: Self::BlockImportOperation) -> BlockchainResult<()> {
        log::warn!("Commit operations are not supported for read-only backend");
        Ok(())
    }

    fn finalize_block(
        &self,
        _block: BlockId<Block>,
        _justification: Option<Justification>,
    ) -> BlockchainResult<()> {
        log::warn!("finalize block operations are not supported for read-only backend");
        Ok(())
    }

    fn append_justification(
        &self,
        _block: BlockId<Block>,
        _justification: Justification,
    ) -> BlockchainResult<()> {
        log::warn!("append justification operations are not supported for read-only backend");
        Ok(())
    }

    fn blockchain(&self) -> &Self::Blockchain {
        &self.blockchain
    }

    fn usage_info(&self) -> Option<UsageInfo> {
        // TODO: Implement usage info (for state reads)
        None
    }

    fn changes_trie_storage(&self) -> Option<&dyn PrunableStateChangesTrieStorage<Block>> {
        // TODO: Implement Changes Trie
        None
    }

    fn offchain_storage(&self) -> Option<Self::OffchainStorage> {
        None
    }

    /// Returns true if state for given block is available.
    fn have_state_at(&self, hash: &Block::Hash, number: NumberFor<Block>) -> bool {
        if self.config.state_pruning.is_archive() {
            match self.blockchain.header_metadata(*hash) {
                Ok(header) => self
                    .storage
                    .get(&header.state_root, (&[], None))
                    .unwrap_or(None)
                    .is_some(),
                Err(_) => false,
            }
        } else {
            !self
                .storage
                .state_db
                .is_pruned(hash, number.saturated_into::<u64>())
        }
    }

    fn state_at(&self, block: BlockId<Block>) -> BlockchainResult<Self::State> {
        let hash = match block {
            // special case for genesis initialization
            BlockId::Hash(h) if h == Default::default() => {
                let genesis_storage = DbGenesisStorage::<Block>::new();
                let root = genesis_storage.0;
                let db_state = DbState::<Block>::new(Arc::new(genesis_storage), root);
                let state = RefTrackingState::new(db_state, self.storage.clone(), None);
                return Ok(state);
            }
            BlockId::Hash(h) => h,
            BlockId::Number(n) => self
                .blockchain
                .hash(n)?
                .ok_or_else(|| unknown_block_err(format!("Unknown block number {}", n)))?,
        };

        match self.blockchain.header_metadata(hash) {
            Ok(header) => {
                if !self.have_state_at(&hash, header.number) {
                    return Err(unknown_block_err(format!(
                        "State already discarded for {:?}",
                        block
                    )));
                }
                if let Ok(()) = self.storage.state_db.pin(&hash) {
                    let root = header.state_root;
                    let db_state = DbState::<Block>::new(self.storage.clone(), root);
                    Ok(RefTrackingState::new(
                        db_state,
                        self.storage.clone(),
                        Some(hash),
                    ))
                } else {
                    Err(unknown_block_err(format!(
                        "State already discarded for {:?}",
                        block
                    )))
                }
            }
            Err(err) => Err(err),
        }
    }

    fn revert(
        &self,
        _n: NumberFor<Block>,
        _revert_finalized: bool,
    ) -> BlockchainResult<(NumberFor<Block>, HashSet<Block::Hash>)> {
        log::warn!("Reverting blocks not supported for a read-only backend");
        Err(backend_err("Reverting blocks not supported"))
    }

    fn get_import_lock(&self) -> &parking_lot::RwLock<()> {
        panic!("No lock exists for read only backend!")
    }
}

impl<Block> AuxStore for ReadOnlyBackend<Block>
where
    Block: BlockT,
{
    fn insert_aux<
        'a,
        'b: 'a,
        'c: 'a,
        I: IntoIterator<Item = &'a (&'c [u8], &'c [u8])>,
        D: IntoIterator<Item = &'a &'b [u8]>,
    >(
        &self,
        _insert: I,
        _delete: D,
    ) -> BlockchainResult<()> {
        log::warn!("Insert aux operations not supported for a read-only backend");
        Ok(())
    }

    fn get_aux(&self, key: &[u8]) -> BlockchainResult<Option<Vec<u8>>> {
        Ok(self.storage.db.get(columns::AUX, key))
    }
}

impl<Block> BlockBackend<Block> for ReadOnlyBackend<Block>
where
    Block: BlockT,
{
    fn block_body(&self, id: &BlockId<Block>) -> BlockchainResult<Option<Vec<Block::Extrinsic>>> {
        self.blockchain().body(*id)
    }

    fn block(&self, id: &BlockId<Block>) -> BlockchainResult<Option<SignedBlock<Block>>> {
        Ok(
            match (
                self.blockchain().header(*id)?,
                self.blockchain().body(*id)?,
                self.blockchain().justifications(*id)?,
            ) {
                (Some(header), Some(extrinsics), justifications) => Some(SignedBlock {
                    block: Block::new(header, extrinsics),
                    justifications,
                }),
                _ => None,
            },
        )
    }

    fn block_status(&self, id: &BlockId<Block>) -> BlockchainResult<BlockStatus> {
        let hash_and_number = match *id {
            BlockId::Hash(hash) => self.blockchain().number(hash)?.map(|number| (hash, number)),
            BlockId::Number(number) => self.blockchain().hash(number)?.map(|hash| (hash, number)),
        };
        match hash_and_number {
            Some((hash, number)) => {
                if self.have_state_at(&hash, number) {
                    Ok(BlockStatus::InChainWithState)
                } else {
                    Ok(BlockStatus::InChainPruned)
                }
            }
            None => Ok(BlockStatus::Unknown),
        }
    }

    fn justifications(&self, id: &BlockId<Block>) -> BlockchainResult<Option<Justifications>> {
        self.blockchain().justifications(*id)
    }

    fn block_hash(&self, number: NumberFor<Block>) -> BlockchainResult<Option<<Block>::Hash>> {
        self.blockchain().hash(number)
    }

    fn indexed_transaction(&self, hash: &Block::Hash) -> BlockchainResult<Option<Vec<u8>>> {
        self.blockchain().indexed_transaction(hash)
    }

    fn has_indexed_transaction(&self, hash: &Block::Hash) -> BlockchainResult<bool> {
        self.blockchain().has_indexed_transaction(hash)
    }
}
