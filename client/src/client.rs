// refer to the client part of sc-service

use std::{marker::PhantomData, panic::UnwindSafe, sync::Arc};

use codec::{Decode, Encode};
use sc_client_api::{
    backend::{self, KeyIterator, PrunableStateChangesTrieStorage, StorageProvider},
    call_executor::{CallExecutor, ExecutorProvider},
    client::BlockBackend,
    execution_extensions::ExecutionExtensions,
};
use sp_api::{
    ApiError, ApiRef, BlockId, CallApiAt, CallApiAtParams, ConstructRuntimeApi, Core as CoreApi,
    Metadata as MetadataApi, NativeOrEncoded, ProvideRuntimeApi,
};
use sp_blockchain::{Backend as BlockchainBackend, HeaderBackend};
use sp_consensus::BlockStatus;
use sp_core::{convert_hash, ChangesTrieConfiguration, OpaqueMetadata};
use sp_runtime::{
    generic::SignedBlock,
    traits::{Block as BlockT, HashFor, Header as HeaderT, NumberFor, One},
    Justifications,
};
use sp_state_machine::{
    key_changes, Backend as StateBackend, ChangesTrieAnchorBlockId, ChangesTrieConfigurationRange,
};
use sp_storage::{well_known_keys, ChildInfo, PrefixedStorageKey, StorageData, StorageKey};
use sp_version::RuntimeVersion;

use crate::error::{BlockchainError, BlockchainResult};

pub struct Client<B, E, Block, RA>
where
    Block: BlockT,
{
    backend: Arc<B>,
    executor: E,
    execution_extensions: ExecutionExtensions<Block>,
    _marker: PhantomData<RA>,
}

impl<B, E, Block, RA> Client<B, E, Block, RA>
where
    B: backend::Backend<Block>,
    E: CallExecutor<Block>,
    Block: BlockT,
{
    pub fn new(
        backend: Arc<B>,
        executor: E,
        execution_extensions: ExecutionExtensions<Block>,
    ) -> Self {
        Self {
            backend,
            executor,
            execution_extensions,
            _marker: PhantomData,
        }
    }

    /// Get the backend of client.
    pub fn backend(&self) -> Arc<B> {
        self.backend.clone()
    }

    /// Get metadata by id.
    pub fn metadata(&self, id: &BlockId<Block>) -> BlockchainResult<OpaqueMetadata>
    where
        E: CallExecutor<Block, Backend = B>,
        RA: ConstructRuntimeApi<Block, Self>,
        <RA as ConstructRuntimeApi<Block, Self>>::RuntimeApi: MetadataApi<Block>,
    {
        self.runtime_api().metadata(id).map_err(Into::into)
    }

    /// Get the RuntimeVersion at a given block.
    pub fn runtime_version_at(&self, id: &BlockId<Block>) -> BlockchainResult<RuntimeVersion> {
        self.executor.runtime_version(id)
    }

    /// Get the code at a given block.
    pub fn code_at(&self, id: BlockId<Block>) -> BlockchainResult<Vec<u8>> {
        Ok(self
            .state_at(id)?
            .storage(well_known_keys::CODE)
            .map_err(|err| BlockchainError::from_state(Box::new(err)))?
            .expect("None is returned if there's no value stored for the given key; ':code' key is always defined; qed"))
    }

    /// Get a reference to the state at a given block.
    pub fn state_at(&self, block: BlockId<Block>) -> BlockchainResult<B::State> {
        self.backend.state_at(block)
    }

    /// Get block status
    pub fn block_status(&self, id: BlockId<Block>) -> BlockchainResult<BlockStatus> {
        let hash_and_number = match id {
            BlockId::Hash(hash) => self
                .backend
                .blockchain()
                .number(hash)?
                .map(|number| (hash, number)),
            BlockId::Number(number) => self
                .backend
                .blockchain()
                .hash(number)?
                .map(|hash| (hash, number)),
        };
        match hash_and_number {
            Some((hash, number)) => {
                if self.backend.have_state_at(&hash, number) {
                    Ok(BlockStatus::InChainWithState)
                } else {
                    Ok(BlockStatus::InChainPruned)
                }
            }
            None => Ok(BlockStatus::Unknown),
        }
    }

    /// Get block header by id.
    pub fn header(&self, id: BlockId<Block>) -> BlockchainResult<Option<Block::Header>> {
        self.backend.blockchain().header(id)
    }

    /// Get block body by id.
    pub fn body(&self, id: BlockId<Block>) -> BlockchainResult<Option<Vec<Block::Extrinsic>>> {
        self.backend.blockchain().body(id)
    }

    /// Get block justifications by id.
    pub fn justifications(&self, id: BlockId<Block>) -> BlockchainResult<Option<Justifications>> {
        self.backend.blockchain().justifications(id)
    }

    /// Returns changes trie storage and all configurations that have been active in the range [first; last].
    ///
    /// Configurations are returned in descending order (and obviously never overlap).
    /// If fail_if_disabled is false, returns maximal consequent configurations ranges, starting from last and
    /// stopping on either first, or when CT have been disabled.
    /// If fail_if_disabled is true, fails when there's a subrange where CT have been disabled
    /// inside first..last blocks range.
    #[allow(clippy::type_complexity)]
    fn require_changes_trie(
        &self,
        first: NumberFor<Block>,
        last: Block::Hash,
        fail_if_disabled: bool,
    ) -> BlockchainResult<(
        &dyn PrunableStateChangesTrieStorage<Block>,
        Vec<(
            NumberFor<Block>,
            Option<(NumberFor<Block>, Block::Hash)>,
            ChangesTrieConfiguration,
        )>,
    )> {
        let storage = match self.backend.changes_trie_storage() {
            Some(storage) => storage,
            None => return Err(BlockchainError::ChangesTriesNotSupported),
        };

        let mut configs = Vec::with_capacity(1);
        let mut current = last;
        loop {
            let config_range = storage.configuration_at(&BlockId::Hash(current))?;
            match config_range.config {
                Some(config) => configs.push((config_range.zero.0, config_range.end, config)),
                None if !fail_if_disabled => return Ok((storage, configs)),
                None => return Err(BlockchainError::ChangesTriesNotSupported),
            }

            if config_range.zero.0 < first {
                break;
            }

            current = *self
                .backend
                .blockchain()
                .expect_header(BlockId::Hash(config_range.zero.1))?
                .parent_hash();
        }

        Ok((storage, configs))
    }

    /// Prepare in-memory header that is used in execution environment.
    fn prepare_environment_block(
        &self,
        parent: &BlockId<Block>,
    ) -> BlockchainResult<Block::Header> {
        let parent_hash = self
            .backend
            .blockchain()
            .expect_block_hash_from_id(parent)?;
        Ok(<<Block as BlockT>::Header as HeaderT>::new(
            self.backend
                .blockchain()
                .expect_block_number_from_id(parent)?
                + One::one(),
            Default::default(),
            Default::default(),
            parent_hash,
            Default::default(),
        ))
    }
}

impl<B, E, Block, RA> ProvideRuntimeApi<Block> for Client<B, E, Block, RA>
where
    B: backend::Backend<Block>,
    E: CallExecutor<Block, Backend = B>,
    Block: BlockT,
    RA: ConstructRuntimeApi<Block, Self>,
{
    type Api = <RA as ConstructRuntimeApi<Block, Self>>::RuntimeApi;

    fn runtime_api(&self) -> ApiRef<'_, Self::Api> {
        RA::construct_runtime_api(self)
    }
}

impl<B, E, Block, RA> CallApiAt<Block> for Client<B, E, Block, RA>
where
    B: backend::Backend<Block>,
    E: CallExecutor<Block, Backend = B>,
    Block: BlockT,
{
    type StateBackend = B::State;

    fn call_api_at<
        R: Encode + Decode + PartialEq,
        NC: FnOnce() -> Result<R, ApiError> + UnwindSafe,
        C: CoreApi<Block>,
    >(
        &self,
        params: CallApiAtParams<'_, Block, C, NC, Self::StateBackend>,
    ) -> Result<NativeOrEncoded<R>, ApiError> {
        let core_api = params.core_api;
        let at = params.at;

        let (manager, extensions) = self
            .execution_extensions
            .manager_and_extensions(at, params.context);

        self.executor
            .contextual_call(
                || {
                    core_api
                        .initialize_block(at, &self.prepare_environment_block(at)?)
                        .map_err(BlockchainError::RuntimeApiError)
                },
                at,
                params.function,
                &params.arguments,
                params.overlayed_changes,
                Some(params.storage_transaction_cache),
                params.initialize_block,
                manager,
                params.native_call,
                params.recorder,
                Some(extensions),
            )
            .map_err(Into::into)
    }

    fn runtime_version_at(&self, at: &BlockId<Block>) -> Result<RuntimeVersion, ApiError> {
        self.runtime_version_at(at).map_err(Into::into)
    }
}

impl<B, E, Block, RA> BlockBackend<Block> for Client<B, E, Block, RA>
where
    B: backend::Backend<Block>,
    E: CallExecutor<Block>,
    Block: BlockT,
{
    fn block_body(&self, id: &BlockId<Block>) -> BlockchainResult<Option<Vec<Block::Extrinsic>>> {
        self.body(*id)
    }

    fn block(&self, id: &BlockId<Block>) -> BlockchainResult<Option<SignedBlock<Block>>> {
        Ok(
            match (
                self.header(*id)?,
                self.body(*id)?,
                self.justifications(*id)?,
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
        self.block_status(*id)
    }

    fn justifications(&self, id: &BlockId<Block>) -> BlockchainResult<Option<Justifications>> {
        self.justifications(*id)
    }

    fn block_hash(&self, number: NumberFor<Block>) -> BlockchainResult<Option<<Block>::Hash>> {
        self.backend.blockchain().hash(number)
    }

    fn indexed_transaction(&self, hash: &Block::Hash) -> BlockchainResult<Option<Vec<u8>>> {
        self.backend.blockchain().indexed_transaction(hash)
    }

    fn has_indexed_transaction(&self, hash: &Block::Hash) -> BlockchainResult<bool> {
        self.backend.blockchain().has_indexed_transaction(hash)
    }
}

impl<Block, B, E, RA> ExecutorProvider<Block> for Client<B, E, Block, RA>
where
    B: backend::Backend<Block>,
    E: CallExecutor<Block>,
    Block: BlockT,
{
    type Executor = E;

    fn executor(&self) -> &Self::Executor {
        &self.executor
    }

    fn execution_extensions(&self) -> &ExecutionExtensions<Block> {
        &self.execution_extensions
    }
}

impl<Block, B, E, RA> StorageProvider<Block, B> for Client<B, E, Block, RA>
where
    B: backend::Backend<Block>,
    E: CallExecutor<Block>,
    Block: BlockT,
{
    fn storage(
        &self,
        id: &BlockId<Block>,
        key: &StorageKey,
    ) -> BlockchainResult<Option<StorageData>> {
        Ok(self
            .state_at(*id)?
            .storage(&key.0)
            .map_err(|err| BlockchainError::from_state(Box::new(err)))?
            .map(StorageData))
    }

    fn storage_keys(
        &self,
        id: &BlockId<Block>,
        key_prefix: &StorageKey,
    ) -> BlockchainResult<Vec<StorageKey>> {
        let state = self.state_at(*id)?;
        Ok(state
            .keys(&key_prefix.0)
            .into_iter()
            .map(StorageKey)
            .collect())
    }

    fn storage_hash(
        &self,
        id: &BlockId<Block>,
        key: &StorageKey,
    ) -> BlockchainResult<Option<Block::Hash>> {
        self.state_at(*id)?
            .storage_hash(&key.0)
            .map_err(|err| BlockchainError::from_state(Box::new(err)))
    }

    fn storage_pairs(
        &self,
        id: &BlockId<Block>,
        key_prefix: &StorageKey,
    ) -> BlockchainResult<Vec<(StorageKey, StorageData)>> {
        let state = self.state_at(*id)?;
        let pairs = state
            .keys(&key_prefix.0)
            .into_iter()
            .map(|key| {
                let data = state.storage(&key).ok().flatten().unwrap_or_default();
                (StorageKey(key), StorageData(data))
            })
            .collect();
        Ok(pairs)
    }

    fn storage_keys_iter<'a>(
        &self,
        id: &BlockId<Block>,
        prefix: Option<&'a StorageKey>,
        start_key: Option<&StorageKey>,
    ) -> BlockchainResult<KeyIterator<'a, B::State, Block>> {
        let state = self.state_at(*id)?;
        let current_key = start_key
            .or(prefix)
            .map(|key| key.0.clone())
            .unwrap_or_default();
        Ok(KeyIterator::new(state, prefix, current_key))
    }

    fn child_storage(
        &self,
        id: &BlockId<Block>,
        child_info: &ChildInfo,
        key: &StorageKey,
    ) -> BlockchainResult<Option<StorageData>> {
        Ok(self
            .state_at(*id)?
            .child_storage(child_info, &key.0)
            .map_err(|err| BlockchainError::from_state(Box::new(err)))?
            .map(StorageData))
    }

    fn child_storage_keys(
        &self,
        id: &BlockId<Block>,
        child_info: &ChildInfo,
        key_prefix: &StorageKey,
    ) -> BlockchainResult<Vec<StorageKey>> {
        let state = self.state_at(*id)?;
        let keys = state
            .child_keys(child_info, &key_prefix.0)
            .into_iter()
            .map(StorageKey)
            .collect();
        Ok(keys)
    }

    fn child_storage_hash(
        &self,
        id: &BlockId<Block>,
        child_info: &ChildInfo,
        key: &StorageKey,
    ) -> BlockchainResult<Option<Block::Hash>> {
        self.state_at(*id)?
            .child_storage_hash(child_info, &key.0)
            .map_err(|err| BlockchainError::from_state(Box::new(err)))
    }

    fn max_key_changes_range(
        &self,
        first: NumberFor<Block>,
        last: BlockId<Block>,
    ) -> BlockchainResult<Option<(NumberFor<Block>, BlockId<Block>)>> {
        let last_number = self
            .backend
            .blockchain()
            .expect_block_number_from_id(&last)?;
        let last_hash = self.backend.blockchain().expect_block_hash_from_id(&last)?;
        if first > last_number {
            return Err(BlockchainError::ChangesTrieAccessFailed(
                "Invalid changes trie range".into(),
            ));
        }

        let (storage, configs) = match self.require_changes_trie(first, last_hash, false).ok() {
            Some((storage, configs)) => (storage, configs),
            None => return Ok(None),
        };

        let first_available_changes_trie = configs.last().map(|config| config.0);
        match first_available_changes_trie {
            Some(first_available_changes_trie) => {
                let oldest_unpruned = storage.oldest_pruned_digest_range_end();
                let first = std::cmp::max(first_available_changes_trie, oldest_unpruned);
                Ok(Some((first, last)))
            }
            None => Ok(None),
        }
    }

    fn key_changes(
        &self,
        first: NumberFor<Block>,
        last: BlockId<Block>,
        storage_key: Option<&PrefixedStorageKey>,
        key: &StorageKey,
    ) -> BlockchainResult<Vec<(NumberFor<Block>, u32)>> {
        let last_number = self
            .backend
            .blockchain()
            .expect_block_number_from_id(&last)?;
        let last_hash = self.backend.blockchain().expect_block_hash_from_id(&last)?;
        let (storage, configs) = self.require_changes_trie(first, last_hash, true)?;

        let mut result = Vec::new();
        let best_number = self.backend.blockchain().info().best_number;
        for (config_zero, config_end, config) in configs {
            let range_first = ::std::cmp::max(first, config_zero + One::one());
            let range_anchor = match config_end {
                Some((config_end_number, config_end_hash)) => {
                    if last_number > config_end_number {
                        ChangesTrieAnchorBlockId {
                            hash: config_end_hash,
                            number: config_end_number,
                        }
                    } else {
                        ChangesTrieAnchorBlockId {
                            hash: convert_hash(&last_hash),
                            number: last_number,
                        }
                    }
                }
                None => ChangesTrieAnchorBlockId {
                    hash: convert_hash(&last_hash),
                    number: last_number,
                },
            };

            let config_range = ChangesTrieConfigurationRange {
                config: &config,
                zero: config_zero,
                end: config_end.map(|(config_end_number, _)| config_end_number),
            };
            let result_range: Vec<(NumberFor<Block>, u32)> = key_changes::<HashFor<Block>, _>(
                config_range,
                storage.storage(),
                range_first,
                &range_anchor,
                best_number,
                storage_key,
                &key.0,
            )
            .and_then(|r| {
                r.map(|r| r.map(|(block, tx)| (block, tx)))
                    .collect::<Result<_, _>>()
            })
            .map_err(BlockchainError::ChangesTrieAccessFailed)?;
            result.extend(result_range);
        }

        Ok(result)
    }
}
