// refer to the client part of sc-service

use std::{marker::PhantomData, panic::UnwindSafe, sync::Arc};

use codec::{Decode, Encode};
use sc_client_api::{
    backend::{self, KeyIterator, StorageProvider},
    call_executor::{CallExecutor, ExecutorProvider},
    execution_extensions::ExecutionExtensions,
};
use sp_api::{
    ApiError, ApiRef, BlockId, CallApiAt, CallApiAtParams, ConstructRuntimeApi,
    Metadata as MetadataApi, NativeOrEncoded, ProvideRuntimeApi,
};
use sp_core::OpaqueMetadata;
use sp_runtime::traits::Block as BlockT;
use sp_state_machine::Backend as StateBackend;
use sp_storage::{well_known_keys, ChildInfo, StorageData, StorageKey};
use sp_version::RuntimeVersion;

use crate::error::{BlockchainError, BlockchainResult};

pub struct Client<Block, Backend, Executor, RA>
where
    Block: BlockT,
{
    backend: Arc<Backend>,
    executor: Executor,
    execution_extensions: ExecutionExtensions<Block>,
    _marker: PhantomData<RA>,
}

impl<Block, Backend, Executor, RA> Client<Block, Backend, Executor, RA>
where
    Backend: backend::Backend<Block>,
    Executor: CallExecutor<Block>,
    Block: BlockT,
{
    pub fn new(
        backend: Arc<Backend>,
        executor: Executor,
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
    pub fn backend(&self) -> Arc<Backend> {
        self.backend.clone()
    }

    /// Get metadata by id.
    pub fn metadata(&self, id: &BlockId<Block>) -> BlockchainResult<OpaqueMetadata>
    where
        Executor: CallExecutor<Block, Backend = Backend>,
        RA: ConstructRuntimeApi<Block, Self>,
        <RA as ConstructRuntimeApi<Block, Self>>::RuntimeApi: MetadataApi<Block>,
    {
        self.runtime_api().metadata(id).map_err(Into::into)
    }

    /// Get the RuntimeVersion by id.
    pub fn runtime_version(&self, id: &BlockId<Block>) -> BlockchainResult<RuntimeVersion> {
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
    pub fn state_at(&self, block: BlockId<Block>) -> BlockchainResult<Backend::State> {
        self.backend.state_at(block)
    }
}

impl<Block, Backend, Executor, RA> ProvideRuntimeApi<Block> for Client<Block, Backend, Executor, RA>
where
    Block: BlockT,
    Backend: backend::Backend<Block>,
    Executor: CallExecutor<Block, Backend = Backend>,
    RA: ConstructRuntimeApi<Block, Self>,
{
    type Api = <RA as ConstructRuntimeApi<Block, Self>>::RuntimeApi;

    fn runtime_api(&self) -> ApiRef<'_, Self::Api> {
        RA::construct_runtime_api(self)
    }
}

impl<Block, Backend, Executor, RA> CallApiAt<Block> for Client<Block, Backend, Executor, RA>
where
    Block: BlockT,
    Backend: backend::Backend<Block>,
    Executor: CallExecutor<Block, Backend = Backend>,
{
    type StateBackend = Backend::State;

    fn call_api_at<
        R: Encode + Decode + PartialEq,
        NC: FnOnce() -> Result<R, ApiError> + UnwindSafe,
    >(
        &self,
        params: CallApiAtParams<'_, Block, NC, Self::StateBackend>,
    ) -> Result<NativeOrEncoded<R>, ApiError> {
        let at = params.at;

        let (manager, extensions) = self
            .execution_extensions
            .manager_and_extensions(at, params.context);

        self.executor
            .contextual_call::<fn(_, _) -> _, _, _>(
                at,
                params.function,
                &params.arguments,
                params.overlayed_changes,
                Some(params.storage_transaction_cache),
                manager,
                params.native_call,
                params.recorder,
                Some(extensions),
            )
            .map_err(Into::into)
    }

    fn runtime_version_at(&self, at: &BlockId<Block>) -> Result<RuntimeVersion, ApiError> {
        self.runtime_version(at).map_err(Into::into)
    }
}

impl<Block, Backend, Executor, RA> ExecutorProvider<Block> for Client<Block, Backend, Executor, RA>
where
    Block: BlockT,
    Backend: backend::Backend<Block>,
    Executor: CallExecutor<Block>,
{
    type Executor = Executor;

    fn executor(&self) -> &Self::Executor {
        &self.executor
    }

    fn execution_extensions(&self) -> &ExecutionExtensions<Block> {
        &self.execution_extensions
    }
}

impl<Block, Backend, Executor, RA> StorageProvider<Block, Backend>
    for Client<Block, Backend, Executor, RA>
where
    Block: BlockT,
    Backend: backend::Backend<Block>,
    Executor: CallExecutor<Block>,
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
    ) -> BlockchainResult<KeyIterator<'a, Backend::State, Block>> {
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

    fn child_storage_keys_iter<'a>(
        &self,
        id: &BlockId<Block>,
        child_info: ChildInfo,
        prefix: Option<&'a StorageKey>,
        start_key: Option<&StorageKey>,
    ) -> BlockchainResult<KeyIterator<'a, Backend::State, Block>> {
        let state = self.state_at(*id)?;
        let start_key = start_key
            .or(prefix)
            .map(|key| key.0.clone())
            .unwrap_or_else(Vec::new);
        Ok(KeyIterator::new_child(state, child_info, prefix, start_key))
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
}
