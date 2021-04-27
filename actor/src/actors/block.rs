use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use xtra::prelude::*;

use sc_client_api::{
    backend::{self, StateBackendFor},
    client::BlockBackend,
};
use sp_api::{ApiExt, BlockId, Core as CoreApi, ProvideRuntimeApi};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor};
use sp_storage::{Storage, StorageData, StorageKey};
use sp_version::RuntimeVersion;

use crate::{
    actors::{metadata::MetadataActor, postgres::PostgresActor},
    error::ActorError,
    exec::BlockExecutor,
    messages::{BlockMessage, Crawl, Die, MaxBlock, ReIndex},
};

pub struct BlockActor<Block, Backend, Api>
where
    Block: BlockT,
{
    backend: Arc<Backend>,
    api: Arc<Api>,
    metadata: Address<MetadataActor<Block>>,
    db: Address<PostgresActor<Block>>,
    genesis: Storage,
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
    pub fn new(
        backend: Arc<Backend>,
        api: Arc<Api>,
        metadata: Address<MetadataActor<Block>>,
        db: Address<PostgresActor<Block>>,
        genesis: Storage,
    ) -> Self {
        Self {
            backend,
            api,
            metadata,
            db,
            genesis,
            curr_block: 0,
        }
    }

    fn genesis_block(&self) -> Result<BlockMessage<Block>, ActorError>
    where
        NumberFor<Block>: From<u32>,
    {
        let id = BlockId::Number(0u32.into());
        let runtime_version = self.api.runtime_api().version(&id)?;
        let block = self
            .backend
            .block(&id)?
            .expect("genesis block must exist; qed");
        Ok(BlockMessage {
            spec_version: runtime_version.spec_version,
            inner: block,
            changes: self
                .genesis
                .top
                .clone()
                .into_iter()
                .map(|(k, v)| (StorageKey(k), Some(StorageData(v))))
                .collect(),
        })
    }

    async fn re_index(&mut self) -> Result<(), ActorError> {
        if let Some(max) = self.db.send(MaxBlock).await? {
            log::info!(target: "actor", "Re-index from the block #{}", max);
            self.curr_block = max + 1;
        } else {
            // `None` means that the blocks table is not populated yet
            log::info!(target: "actor", "Re-index from the genesis block");
            let genesis_block = self.genesis_block()?;
            self.metadata.send(genesis_block).await?;
            self.curr_block = 1;
        }
        Ok(())
    }

    async fn crawl(&mut self) -> Result<(), ActorError> {
        let id = BlockId::Number(self.curr_block.into());
        if let Some(block) = self.backend.block(&id)? {
            log::info!(
                target: "actor",
                "Crawl block #{}, hash = {:?}",
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
            log::debug!(target: "actor", "Took {:?} to execute block", now.elapsed());

            let message = BlockMessage {
                spec_version: runtime_version.spec_version,
                inner: block,
                changes: changes.main_storage_changes,
            };
            self.metadata.send(message).await?;
            self.curr_block += 1;
            Ok(())
        } else {
            tokio::time::sleep(Duration::from_secs(1)).await;
            Ok(())
        }
    }
}

#[async_trait::async_trait]
impl<Block, Backend, Api> Actor for BlockActor<Block, Backend, Api>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + BlockBackend<Block> + 'static,
    Api: ProvideRuntimeApi<Block> + Send + Sync + 'static,
    <Api as ProvideRuntimeApi<Block>>::Api:
        CoreApi<Block> + ApiExt<Block, StateBackend = StateBackendFor<Backend, Block>>,
{
    async fn started(&mut self, ctx: &mut Context<Self>) {
        // using this instead of notify_immediately because
        // ReIndexing is async process
        let addr = ctx.address().expect("Actor just started");
        addr.do_send(ReIndex).expect("Actor just started");

        tokio::task::spawn(async move {
            loop {
                if addr.send(Crawl).await.is_err() {
                    log::error!(target: "actor", "Crawl block error");
                    break;
                }
            }
        });
    }
}

#[async_trait::async_trait]
impl<Block, Backend, Api> Handler<ReIndex> for BlockActor<Block, Backend, Api>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + BlockBackend<Block> + 'static,
    Api: ProvideRuntimeApi<Block> + Send + Sync + 'static,
    <Api as ProvideRuntimeApi<Block>>::Api:
        CoreApi<Block> + ApiExt<Block, StateBackend = StateBackendFor<Backend, Block>>,
{
    async fn handle(
        &mut self,
        _: ReIndex,
        ctx: &mut Context<Self>,
    ) -> <ReIndex as Message>::Result {
        match self.re_index().await {
            Ok(()) => {}
            Err(err) => {
                log::error!(target: "actor", "{}", err);
                ctx.stop();
            }
        }
    }
}

#[async_trait::async_trait]
impl<Block, Backend, Api> Handler<Crawl> for BlockActor<Block, Backend, Api>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + BlockBackend<Block> + 'static,
    Api: ProvideRuntimeApi<Block> + Send + Sync + 'static,
    <Api as ProvideRuntimeApi<Block>>::Api:
        CoreApi<Block> + ApiExt<Block, StateBackend = StateBackendFor<Backend, Block>>,
{
    async fn handle(&mut self, _: Crawl, ctx: &mut Context<Self>) -> <Crawl as Message>::Result {
        match self.crawl().await {
            Ok(()) => {}
            Err(err) => {
                log::error!(target: "actor", "{}", err);
                ctx.stop();
            }
        }
    }
}

#[async_trait::async_trait]
impl<Block, Backend, Api> Handler<Die> for BlockActor<Block, Backend, Api>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + BlockBackend<Block> + 'static,
    Api: ProvideRuntimeApi<Block> + Send + Sync + 'static,
    <Api as ProvideRuntimeApi<Block>>::Api:
        CoreApi<Block> + ApiExt<Block, StateBackend = StateBackendFor<Backend, Block>>,
{
    async fn handle(&mut self, _: Die, ctx: &mut Context<Self>) -> <Die as Message>::Result {
        log::info!(target: "actor", "Stopping Block Actor");
        ctx.stop();
    }
}
