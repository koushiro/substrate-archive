use std::{marker::PhantomData, sync::Arc, time::Instant};

use xtra::prelude::*;

use sc_client_api::{
    backend::{self, StateBackendFor},
    client::BlockBackend,
};
use sp_api::{ApiExt, BlockId, Core as CoreApi, ProvideRuntimeApi};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use sp_version::RuntimeVersion;

use crate::{
    error::ActorError,
    exec::BlockExecutor,
    message::{BlockMessage, CrawlBlock, Die},
};

pub struct BlockActor<Block: BlockT, Backend, Api> {
    _marker: PhantomData<Block>,
    backend: Arc<Backend>,
    api: Arc<Api>,
    curr_block: u32,
}

impl<Block, Backend, Api> BlockActor<Block, Backend, Api>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + BlockBackend<Block>,
    Api: ProvideRuntimeApi<Block>,
    <Api as ProvideRuntimeApi<Block>>::Api:
        CoreApi<Block> + ApiExt<Block, StateBackend = StateBackendFor<Backend, Block>>,
{
    pub fn new(backend: Arc<Backend>, api: Arc<Api>) -> Self {
        Self {
            _marker: PhantomData,
            backend,
            api,
            curr_block: 0,
        }
    }

    async fn crawl(
        &mut self,
        message: CrawlBlock<Block>,
    ) -> Result<Option<BlockMessage<Block>>, ActorError> {
        self.curr_block = message.block_num();
        let id = BlockId::Number(self.curr_block.into());
        if let Some(block) = self.backend.block(&id)? {
            log::info!(
                target: "actor",
                "Crawl Block #{}, hash = {:?}",
                block.block.header().number(),
                block.block.header().hash()
            );

            let api = self.api.runtime_api();
            let runtime_version: RuntimeVersion = api.version(&id)?;
            log::debug!(
                target: "actor",
                "Executing Block #{} ({}), version {}",
                block.block.header().number(),
                block.block.header().hash(),
                runtime_version.spec_version
            );

            let now = Instant::now();
            // Must re-construct a new runtime api for executing block.
            let api = self.api.runtime_api();
            let executor = BlockExecutor::new(block.block.clone(), &self.backend, api);
            let changes = executor.into_storage_changes()?;
            log::debug!(
                target: "actor",
                "Took {:?} to execute block #{}",
                now.elapsed(), block.block.header().number()
            );

            Ok(Some(BlockMessage {
                spec_version: runtime_version.spec_version,
                inner: block,
                main_changes: changes.main_storage_changes,
                child_changes: changes.child_storage_changes,
            }))
        } else {
            Ok(None)
        }
    }
}

#[async_trait::async_trait]
impl<Block, Backend, Api> Actor for BlockActor<Block, Backend, Api>
where
    Block: BlockT,
    Backend: Send + Sync + 'static,
    Api: Send + Sync + 'static,
{
}

#[async_trait::async_trait]
impl<Block, Backend, Api> Handler<CrawlBlock<Block>> for BlockActor<Block, Backend, Api>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + BlockBackend<Block> + 'static,
    Api: ProvideRuntimeApi<Block> + Send + Sync + 'static,
    <Api as ProvideRuntimeApi<Block>>::Api:
        CoreApi<Block> + ApiExt<Block, StateBackend = StateBackendFor<Backend, Block>>,
{
    async fn handle(
        &mut self,
        message: CrawlBlock<Block>,
        _: &mut Context<Self>,
    ) -> <CrawlBlock<Block> as Message>::Result {
        match self.crawl(message).await {
            Ok(block) => block,
            // If error occurred, don't stop the actor.
            Err(err) => {
                log::error!(target: "actor", "{}", err);
                None
            }
        }
    }
}

#[async_trait::async_trait]
impl<Block, Backend, Api> Handler<Die> for BlockActor<Block, Backend, Api>
where
    Block: BlockT,
    Backend: Send + Sync + 'static,
    Api: Send + Sync + 'static,
{
    async fn handle(&mut self, _: Die, ctx: &mut Context<Self>) -> <Die as Message>::Result {
        log::info!(target: "actor", "Stopping Block Actor");
        ctx.stop();
    }
}
