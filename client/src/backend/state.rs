use std::{fmt, sync::Arc};

use hash_db::Prefix;
use kvdb::DBValue;
use memory_db::prefixed_key;

use sc_state_db::{MetaDb, NodeDb, StateDb};
use sp_runtime::traits::{Block as BlockT, HashFor};
use sp_state_machine::{
    Backend as StateBackend, DefaultError, MemoryDB, StateMachineStats, Storage, TrieBackend,
    TrieDBMut, UsageInfo,
};
use sp_storage::ChildInfo;

use crate::{columns, database::ReadOnlyDB};

/// DB-backed patricia trie state, transaction type is an overlay of changes to commit.
pub type DbState<Block> = TrieBackend<Arc<dyn Storage<HashFor<Block>>>, HashFor<Block>>;

pub struct StorageDb<Block: BlockT> {
    pub db: Arc<dyn ReadOnlyDB>,
    pub state_db: StateDb<Block::Hash, Vec<u8>>,
    pub prefix_keys: bool,
}

impl<Block: BlockT> Storage<HashFor<Block>> for StorageDb<Block> {
    fn get(&self, key: &Block::Hash, prefix: Prefix) -> Result<Option<DBValue>, DefaultError> {
        if self.prefix_keys {
            let key = prefixed_key::<HashFor<Block>>(key, prefix);
            self.state_db.get(&key, self)
        } else {
            self.state_db.get(key.as_ref(), self)
        }
        .map_err(|e| format!("Database backend error: {:?}", e))
    }
}

impl<Block: BlockT> NodeDb for StorageDb<Block> {
    type Key = [u8];
    type Error = DefaultError;

    fn get(&self, key: &Self::Key) -> Result<Option<DBValue>, Self::Error> {
        Ok(self.db.get(columns::STATE, key))
    }
}

// wrapper that implements trait required for state_db
pub struct StateMetaDb<'a>(pub &'a dyn ReadOnlyDB);
impl<'a> MetaDb for StateMetaDb<'a> {
    type Error = DefaultError;

    fn get_meta(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self.0.get(columns::STATE_META, key))
    }
}

/// A reference tracking state.
///
/// It makes sure that the hash we are using stays pinned in storage
/// until this structure is dropped.
pub struct RefTrackingState<Block: BlockT> {
    state: DbState<Block>,
    storage: Arc<StorageDb<Block>>,
    parent_hash: Option<Block::Hash>,
}

impl<Block: BlockT> fmt::Debug for RefTrackingState<Block> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Block {:?}", self.parent_hash)
    }
}

impl<B: BlockT> Drop for RefTrackingState<B> {
    fn drop(&mut self) {
        if let Some(hash) = &self.parent_hash {
            self.storage.state_db.unpin(hash);
        }
    }
}

impl<B: BlockT> RefTrackingState<B> {
    pub fn new(
        state: DbState<B>,
        storage: Arc<StorageDb<B>>,
        parent_hash: Option<B::Hash>,
    ) -> Self {
        RefTrackingState {
            state,
            parent_hash,
            storage,
        }
    }
}

impl<B: BlockT> StateBackend<HashFor<B>> for RefTrackingState<B> {
    type Error = <DbState<B> as StateBackend<HashFor<B>>>::Error;
    type Transaction = <DbState<B> as StateBackend<HashFor<B>>>::Transaction;
    type TrieBackendStorage = <DbState<B> as StateBackend<HashFor<B>>>::TrieBackendStorage;

    fn storage(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        self.state.storage(key)
    }

    fn storage_hash(&self, key: &[u8]) -> Result<Option<B::Hash>, Self::Error> {
        self.state.storage_hash(key)
    }

    fn child_storage(
        &self,
        child_info: &ChildInfo,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        self.state.child_storage(child_info, key)
    }

    fn exists_storage(&self, key: &[u8]) -> Result<bool, Self::Error> {
        self.state.exists_storage(key)
    }

    fn exists_child_storage(
        &self,
        child_info: &ChildInfo,
        key: &[u8],
    ) -> Result<bool, Self::Error> {
        self.state.exists_child_storage(child_info, key)
    }

    fn next_storage_key(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        self.state.next_storage_key(key)
    }

    fn next_child_storage_key(
        &self,
        child_info: &ChildInfo,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        self.state.next_child_storage_key(child_info, key)
    }

    fn apply_to_child_keys_while<F: FnMut(&[u8]) -> bool>(&self, child_info: &ChildInfo, f: F) {
        self.state.apply_to_child_keys_while(child_info, f)
    }

    fn for_keys_with_prefix<F: FnMut(&[u8])>(&self, prefix: &[u8], f: F) {
        self.state.for_keys_with_prefix(prefix, f)
    }

    fn for_key_values_with_prefix<F: FnMut(&[u8], &[u8])>(&self, prefix: &[u8], f: F) {
        self.state.for_key_values_with_prefix(prefix, f)
    }

    fn for_child_keys_with_prefix<F: FnMut(&[u8])>(
        &self,
        child_info: &ChildInfo,
        prefix: &[u8],
        f: F,
    ) {
        self.state.for_child_keys_with_prefix(child_info, prefix, f)
    }

    fn storage_root<'a>(
        &self,
        delta: impl Iterator<Item = (&'a [u8], Option<&'a [u8]>)>,
    ) -> (B::Hash, Self::Transaction)
    where
        B::Hash: Ord,
    {
        self.state.storage_root(delta)
    }

    fn child_storage_root<'a>(
        &self,
        child_info: &ChildInfo,
        delta: impl Iterator<Item = (&'a [u8], Option<&'a [u8]>)>,
    ) -> (B::Hash, bool, Self::Transaction)
    where
        B::Hash: Ord,
    {
        self.state.child_storage_root(child_info, delta)
    }

    fn pairs(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.state.pairs()
    }

    fn keys(&self, prefix: &[u8]) -> Vec<Vec<u8>> {
        self.state.keys(prefix)
    }

    fn child_keys(&self, child_info: &ChildInfo, prefix: &[u8]) -> Vec<Vec<u8>> {
        self.state.child_keys(child_info, prefix)
    }

    fn as_trie_backend(&mut self) -> Option<&TrieBackend<Self::TrieBackendStorage, HashFor<B>>> {
        self.state.as_trie_backend()
    }

    fn register_overlay_stats(&mut self, stats: &StateMachineStats) {
        self.state.register_overlay_stats(stats);
    }

    fn usage_info(&self) -> UsageInfo {
        self.state.usage_info()
    }
}

pub struct DbGenesisStorage<Block: BlockT>(pub Block::Hash);
impl<Block: BlockT> DbGenesisStorage<Block> {
    pub fn new() -> Self {
        let mut root = Block::Hash::default();
        let mut mdb = MemoryDB::<HashFor<Block>>::default();
        TrieDBMut::<HashFor<Block>>::new(&mut mdb, &mut root);
        DbGenesisStorage(root)
    }
}

impl<Block: BlockT> Storage<HashFor<Block>> for DbGenesisStorage<Block> {
    fn get(&self, _key: &Block::Hash, _prefix: Prefix) -> Result<Option<DBValue>, DefaultError> {
        Ok(None)
    }
}
