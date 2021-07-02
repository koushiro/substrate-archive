mod best_finalized;
mod block;

use std::sync::Arc;

use xtra::{prelude::*, spawn::TokioGlobalSpawnExt};

use sc_client_api::{
    backend::{self, StateBackendFor},
    client::BlockBackend,
};
use sp_api::{ApiExt, BlockId, Core as CoreApi, ProvideRuntimeApi};
use sp_runtime::traits::{Block as BlockT, NumberFor};
use sp_storage::Storage;

use self::{best_finalized::BestAndFinalizedActor, block::BlockActor};
use crate::{
    actors::{metadata::MetadataActor, postgres::PostgresActor},
    error::ActorError,
    message::*,
};

pub struct Scheduler<Block, Backend, Api>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + 'static,
    Api: Send + Sync + 'static,
{
    genesis: Storage,

    backend: Arc<Backend>,
    api: Arc<Api>,

    db: Address<PostgresActor<Block>>,
    metadata: Address<MetadataActor<Block>>,

    best_and_finalized: Address<BestAndFinalizedActor<Block, Backend>>,
    block: Address<BlockActor<Block, Backend, Api>>,

    curr_block: u32,
    best_block: u32,
    finalized_block: u32,
}

impl<Block, Backend, Api> Scheduler<Block, Backend, Api>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + BlockBackend<Block>,
    Api: ProvideRuntimeApi<Block> + Send + Sync + 'static,
    <Api as ProvideRuntimeApi<Block>>::Api:
        CoreApi<Block> + ApiExt<Block, StateBackend = StateBackendFor<Backend, Block>>,
{
    pub fn new(
        genesis: Storage,
        backend: Arc<Backend>,
        api: Arc<Api>,
        db: Address<PostgresActor<Block>>,
        metadata: Address<MetadataActor<Block>>,
    ) -> Self {
        let best_and_finalized =
            BestAndFinalizedActor::<Block, Backend>::new(backend.clone(), db.clone(), 1000)
                .create(None)
                .spawn_global();
        log::info!(target: "actor", "Spawn BestAndFinalized Actor");

        let block = BlockActor::<Block, Backend, Api>::new(backend.clone(), api.clone())
            .create(None)
            .spawn_global();
        log::info!(target: "actor", "Spawn Block Actor");

        Self {
            genesis,
            backend,
            api,
            db,
            metadata,
            best_and_finalized,
            block,
            curr_block: 0,
            best_block: 0,
            finalized_block: 0,
        }
    }

    async fn initialize(&mut self) -> Result<(), ActorError> {
        // https://github.com/rust-lang/rust/issues/71126
        let (best_block, finalized_block) = self.best_and_finalized().await?;
        self.best_block = best_block;
        self.finalized_block = finalized_block;

        if let Some(max) = self.db.send(DbMaxBlock).await? {
            self.curr_block = std::cmp::min(max, self.finalized_block);
            // TODO: remove the blocks and storages (Block finalized_block ~ Block max) from db if max > finalized_block
            log::info!(target: "actor", "Initialize from the Block #{}", self.curr_block);
        } else {
            // `None` means that the blocks table is not populated yet
            log::info!(target: "actor", "Initialize from the Genesis Block");
            let genesis_block = self.genesis_block()?;
            self.metadata.send(genesis_block).await?;
        }
        Ok(())
    }

    async fn best_and_finalized(&mut self) -> Result<(u32, u32), ActorError> {
        Ok(self.best_and_finalized.send(BestAndFinalized).await?)
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
        let genesis = self.genesis.clone();
        Ok(BlockMessage {
            spec_version: runtime_version.spec_version,
            inner: block,
            main_changes: genesis.top.into_iter().map(|(k, v)| (k, Some(v))).collect(),
            child_changes: genesis
                .children_default
                .into_iter()
                .map(|(k, child)| {
                    (
                        k,
                        child
                            .data
                            .into_iter()
                            .map(|(k, v)| (k, Some(v)))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect(),
        })
    }

    async fn tick(&mut self) -> Result<(), ActorError> {
        self.curr_block += 1;
        if let Some(block) = self.block.send(CrawlBlock::new(self.curr_block)).await? {
            self.metadata.send(block).await?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl<Block, Backend, Api> Actor for Scheduler<Block, Backend, Api>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + 'static,
    Api: Send + Sync + 'static,
{
}

#[async_trait::async_trait]
impl<Block, Backend, Api> Handler<Initialize> for Scheduler<Block, Backend, Api>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + BlockBackend<Block> + 'static,
    Api: ProvideRuntimeApi<Block> + Send + Sync + 'static,
    <Api as ProvideRuntimeApi<Block>>::Api:
        CoreApi<Block> + ApiExt<Block, StateBackend = StateBackendFor<Backend, Block>>,
{
    async fn handle(
        &mut self,
        _: Initialize,
        ctx: &mut Context<Self>,
    ) -> <Initialize as Message>::Result {
        match self.initialize().await {
            Ok(()) => {}
            Err(err) => {
                log::error!(target: "actor", "Initialize Scheduler Actor: {}", err);
                ctx.stop();
            }
        }
    }
}

#[async_trait::async_trait]
impl<Block, Backend, Api> Handler<Tick> for Scheduler<Block, Backend, Api>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + BlockBackend<Block> + 'static,
    Api: ProvideRuntimeApi<Block> + Send + Sync + 'static,
    <Api as ProvideRuntimeApi<Block>>::Api:
        CoreApi<Block> + ApiExt<Block, StateBackend = StateBackendFor<Backend, Block>>,
{
    async fn handle(&mut self, _: Tick, _ctx: &mut Context<Self>) -> <Tick as Message>::Result {
        self.tick().await
    }
}

#[async_trait::async_trait]
impl<Block, Backend, Api> Handler<Die> for Scheduler<Block, Backend, Api>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + 'static,
    Api: Send + Sync + 'static,
{
    async fn handle(&mut self, _: Die, ctx: &mut Context<Self>) -> <Die as Message>::Result {
        log::info!(target: "actor", "Stopping Scheduler Actor");
        if let Err(_) = self.block.send(Die).await {
            log::error!(target: "actor", "Stop Block Actor But Disconnected");
        }
        log::info!(target: "actor", "Stopped Block Actor");
        if let Err(_) = self.best_and_finalized.send(Die).await {
            log::error!(target: "actor", "Stop BestAndFinalized Actor But Disconnected");
        }
        log::info!(target: "actor", "Stopped BestAndFinalized Actor");
        ctx.stop();
    }
}
