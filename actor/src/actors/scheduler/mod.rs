mod best_finalized;
mod block;

use std::{cmp, sync::Arc, time::Duration};

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
    config::SchedulerConfig,
    error::ActorError,
    message::*,
};

pub struct Scheduler<Block, Backend, Api>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + 'static,
    Api: Send + Sync + 'static,
{
    backend: Arc<Backend>,
    api: Arc<Api>,

    db: Address<PostgresActor<Block>>,
    metadata: Address<MetadataActor<Block>>,

    best_and_finalized: Address<BestAndFinalizedActor<Block, Backend>>,
    blocks: Vec<Address<BlockActor<Block, Backend, Api>>>,

    genesis: Storage,
    config: SchedulerConfig,
    curr_block: u32,
    catchup_finalized: bool,
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
        backend: Arc<Backend>,
        api: Arc<Api>,
        db: Address<PostgresActor<Block>>,
        metadata: Address<MetadataActor<Block>>,
        genesis: Storage,
        config: SchedulerConfig,
    ) -> Self {
        let best_and_finalized = BestAndFinalizedActor::<Block, Backend>::new(
            backend.clone(),
            db.clone(),
            config.interval_ms,
        )
        .create(None)
        .spawn_global();
        log::info!(target: "actor", "Spawn BestAndFinalized Actor");

        let mut blocks = Vec::with_capacity(config.max_block_load as usize);
        assert!(config.max_block_load >= 1, "max_block_load must be >= 1");
        for i in 0..config.max_block_load {
            blocks.push(
                BlockActor::<Block, Backend, Api>::new(backend.clone(), api.clone())
                    .create(None)
                    .spawn_global(),
            );
            log::info!(target: "actor", "Spawn Block[{}] Actor", i);
        }
        let curr_block = config.start_block.unwrap_or_default();

        Self {
            backend,
            api,

            db,
            metadata,

            best_and_finalized,
            blocks,

            genesis,
            config,
            curr_block,
            catchup_finalized: false,
        }
    }

    async fn initialize(&mut self) -> Result<(), ActorError> {
        let (_best_block, finalized_block) = self.best_and_finalized().await?;
        if let Some(max) = self.db.send(DbMaxBlock).await? {
            if let Some(start_block) = self.config.start_block {
                self.curr_block = cmp::min(cmp::min(max, finalized_block), start_block);
            } else {
                self.curr_block = cmp::min(max, finalized_block);
            }
            // TODO: remove the blocks and storages (Block #curr_block ~ Block #max) from db if max > curr_block
        } else {
            // `None` means that the blocks table is empty yet
            let genesis_block = self.genesis_block()?;
            self.metadata.send(genesis_block).await?;
        }
        log::info!(target: "actor", "Initialize from the Block #{}", self.curr_block);
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
        let (_best_block, finalized_block) = self.best_and_finalized().await?;
        if self.curr_block + self.config.max_block_load <= finalized_block {
            // Haven't caught up with the latest finalized block
            self.tick_batch().await?;
        } else {
            // It's about to catch up with the latest finalized block
            self.tick_one().await?;
        }

        // start to dispatch finalized block
        if !self.catchup_finalized && self.curr_block > finalized_block {
            log::info!(
                target: "actor", "Scheduler catchup the finalized block (curr #{}, finalized #{})",
                self.curr_block, finalized_block
            );
            self.catchup_finalized = true;
            self.db.send(CatchupFinalized).await?;
        }

        Ok(())
    }

    async fn tick_one(&mut self) -> Result<(), ActorError> {
        let next_block = self.curr_block + 1;
        let block = self.blocks.first().expect("At least one block actor");
        log::debug!(target: "actor", "Block[0] Actor Crawling Block #{}", next_block);
        // TODO: check the forked block
        if let Some(block) = block.send(CrawlBlock::new(next_block)).await? {
            self.metadata.send(block).await?;
            self.curr_block = next_block;
        } else {
            tokio::time::sleep(Duration::from_millis(self.config.interval_ms)).await;
        }
        Ok(())
    }

    async fn tick_batch(&mut self) -> Result<(), ActorError> {
        // example:
        // curr_block = 0; max_block_load = 10  => (0+1..0+10] => (1..10]
        // curr_block = 10; max_block_load = 10 => (10+1..10+10] => (11..20]
        let fut = ((self.curr_block + 1)..=(self.curr_block + self.config.max_block_load))
            .map(|i| {
                let index = (i % self.config.max_block_load) as usize;
                log::debug!(target: "actor", "Block[{}] Actor Crawling Block #{}", index, i);
                self.blocks[index].send(CrawlBlock::new(i))
            })
            .collect::<Vec<_>>();
        let results = futures::future::join_all(fut).await;
        let results = results
            .into_iter()
            .map(|result| result.unwrap().unwrap()) // shouldn't be none or error
            .collect::<Vec<_>>();
        self.metadata.send(BatchBlockMessage::new(results)).await?;
        self.curr_block += self.config.max_block_load;
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
        for (index, block) in self.blocks.iter().enumerate() {
            if let Err(_) = block.send(Die).await {
                log::error!(target: "actor", "Stop Block[{}] Actor But Disconnected", index);
            }
            log::info!(target: "actor", "Stopped Block[{}] Actor", index);
        }
        if let Err(_) = self.best_and_finalized.send(Die).await {
            log::error!(target: "actor", "Stop BestAndFinalized Actor But Disconnected");
        }
        log::info!(target: "actor", "Stopped BestAndFinalized Actor");
        ctx.stop();
    }
}
