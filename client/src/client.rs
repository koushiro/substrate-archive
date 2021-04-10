// refer to the client part of sc-service

use std::{marker::PhantomData, panic::UnwindSafe, sync::Arc};

use codec::{Decode, Encode};
use sc_client_api::{
    backend::{self, KeyIterator, StorageProvider},
    execution_extensions::ExecutionExtensions,
    CallExecutor,
};
use sc_service::ClientConfig;
use sp_api::{
    ApiError, ApiRef, BlockId, CallApiAt, CallApiAtParams, ConstructRuntimeApi, Core as CoreApi,
    Metadata as MetadataApi, NativeOrEncoded, ProvideRuntimeApi,
};
use sp_blockchain::{Backend as BlockchainBackend, HeaderBackend};
use sp_core::OpaqueMetadata;
use sp_runtime::{
    traits::{Block as BlockT, Header as HeaderT, NumberFor, One},
    Justifications,
};
use sp_state_machine::Backend as StateBackend;
use sp_storage::{ChildInfo, PrefixedStorageKey, StorageData, StorageKey};
use sp_version::RuntimeVersion;

use crate::error::{BlockchainError, BlockchainResult};

pub struct Client<B, E, Block, RA>
where
    Block: BlockT,
{
    backend: Arc<B>,
    executor: E,
    execution_extensions: ExecutionExtensions<Block>,
    #[allow(dead_code)]
    config: ClientConfig,
    _marker: PhantomData<RA>,
}

impl<B, E, Block, RA> Client<B, E, Block, RA>
where
    B: backend::Backend<Block>,
    E: CallExecutor<Block, Backend = B>,
    Block: BlockT,
{
    pub fn new(
        backend: Arc<B>,
        executor: E,
        execution_extensions: ExecutionExtensions<Block>,
        config: ClientConfig,
    ) -> Self {
        Self {
            backend,
            executor,
            execution_extensions,
            config,
            _marker: PhantomData,
        }
    }

    /// Get metadata by id.
    pub fn metadata(&self, id: &BlockId<Block>) -> BlockchainResult<OpaqueMetadata>
    where
        RA: ConstructRuntimeApi<Block, Self>,
        <RA as ConstructRuntimeApi<Block, Self>>::RuntimeApi: MetadataApi<Block>,
    {
        self.runtime_api().metadata(id).map_err(Into::into)
    }

    /// Get a reference to the state at a given block.
    pub fn state_at(&self, block: &BlockId<Block>) -> BlockchainResult<B::State> {
        self.backend.state_at(*block)
    }

    /// Get the RuntimeVersion at a given block.
    pub fn runtime_version_at(&self, id: &BlockId<Block>) -> BlockchainResult<RuntimeVersion> {
        self.executor.runtime_version(id)
    }

    /// Get block header by id.
    pub fn header(&self, id: &BlockId<Block>) -> BlockchainResult<Option<Block::Header>> {
        self.backend.blockchain().header(*id)
    }

    /// Get block body by id.
    pub fn body(&self, id: &BlockId<Block>) -> BlockchainResult<Option<Vec<Block::Extrinsic>>> {
        self.backend.blockchain().body(*id)
    }

    /// Get block justifications by id.
    pub fn justifications(&self, id: &BlockId<Block>) -> BlockchainResult<Option<Justifications>> {
        self.backend.blockchain().justifications(*id)
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

impl<Block, B, E, RA> StorageProvider<Block, B> for Client<B, E, Block, RA>
where
    B: backend::Backend<Block>,
    E: CallExecutor<Block, Backend = B>,
    Block: BlockT,
{
    fn storage(
        &self,
        id: &BlockId<Block>,
        key: &StorageKey,
    ) -> BlockchainResult<Option<StorageData>> {
        let state = self.state_at(id)?;
        Ok(state
            .storage(&key.0)
            .map_err(|err| BlockchainError::from_state(Box::new(err)))?
            .map(StorageData))
    }

    fn storage_keys(
        &self,
        id: &BlockId<Block>,
        key_prefix: &StorageKey,
    ) -> BlockchainResult<Vec<StorageKey>> {
        let state = self.state_at(id)?;
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
        let state = self.state_at(id)?;
        state
            .storage_hash(&key.0)
            .map_err(|err| BlockchainError::from_state(Box::new(err)))
    }

    fn storage_pairs(
        &self,
        id: &BlockId<Block>,
        key_prefix: &StorageKey,
    ) -> BlockchainResult<Vec<(StorageKey, StorageData)>> {
        let state = self.state_at(id)?;
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
        let state = self.state_at(id)?;
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
        let state = self.state_at(id)?;
        Ok(state
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
        let state = self.state_at(id)?;
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
        let state = self.state_at(id)?;
        state
            .child_storage_hash(child_info, &key.0)
            .map_err(|err| BlockchainError::from_state(Box::new(err)))
    }

    fn max_key_changes_range(
        &self,
        _first: NumberFor<Block>,
        _last: BlockId<Block>,
    ) -> BlockchainResult<Option<(NumberFor<Block>, BlockId<Block>)>> {
        unimplemented!()
    }

    fn key_changes(
        &self,
        _first: NumberFor<Block>,
        _last: BlockId<Block>,
        _storage_key: Option<&PrefixedStorageKey>,
        _key: &StorageKey,
    ) -> BlockchainResult<Vec<(NumberFor<Block>, u32)>> {
        unimplemented!()
    }
}
