mod best_finalized;
mod block;

use std::{cmp, collections::BTreeMap, sync::Arc, time::Duration};

use xtra::{prelude::*, spawn::TokioGlobalSpawnExt};

use sc_client_api::{
    backend::{self, StateBackendFor},
    client::BlockBackend,
};
use sp_api::{ApiExt, BlockId, Core as CoreApi, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_runtime::{
    generic::SignedBlock,
    traits::{Block as BlockT, Header as HeaderT, NumberFor},
};
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

    // curr_finalized_block                   curr_block
    //    |                                       |
    // +--+--+    +-----+    +-----+           +--+--+
    // |     |◄---+     |◄---+     |◄---...◄---+     |
    // +--+--+    +--+--+    +-----+           +--+--+
    //    |                                       |
    //    +---------------------------------------+
    //                    queue
    queue: BTreeMap<u32, Block::Header>,
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

            queue: Default::default(),
        }
    }

    // curr_block(curr_finalized_block)
    //    |
    // +--+--+
    // |     |◄---  queue: [curr_finalized_block]
    // +-----+
    async fn initialize(&mut self) -> Result<(), ActorError> {
        let (_best_block, finalized_block) = self.best_and_finalized().await?;
        if let Some(max) = self.db.send(DbMaxBlock).await?? {
            if let Some(start_block) = self.config.start_block {
                self.curr_block = cmp::min(finalized_block, start_block);
            } else {
                self.curr_block = cmp::min(max, finalized_block);
            }
            // delete the metadatas, blocks and storages (block_num > curr_block) from db
            let _ = self
                .db
                .send(DbDeleteGtBlockNum::new(self.curr_block))
                .await??;
        } else {
            // `None` means that the blocks table is empty yet
            let genesis_block = self.genesis_block()?;
            self.metadata.send(genesis_block).await?;
        }

        // initialized block must exist
        let curr_finalized_block = self
            .header(self.curr_block)?
            .expect("initialized block must exist");
        self.queue.insert(self.curr_block, curr_finalized_block);
        log::info!(
            target: "actor",
            "Initialize scheduler from the Finalized Block #{}",
            self.curr_block
        );
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
        let block = self.block(&id)?.expect("genesis block must exist; qed");
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

    fn block(&self, id: &BlockId<Block>) -> Result<Option<SignedBlock<Block>>, ActorError> {
        Ok(self.backend.block(&id)?)
    }

    fn header(&self, block_num: u32) -> Result<Option<Block::Header>, ActorError> {
        Ok(self.backend.blockchain().header(BlockId::Number(
            <Block::Header as HeaderT>::Number::from(block_num),
        ))?)
    }

    fn curr_finalized_block_num(&self) -> u32 {
        self.queue
            .keys()
            .min()
            .map(|h| *h)
            .expect("At least one finalized block exist")
    }

    fn queue_back_block_hash(&self) -> Block::Hash {
        let max = self
            .queue
            .keys()
            .max()
            .map(|h| *h)
            .expect("At least one finalized block exist");
        self.queue
            .get(&max)
            .expect("At least one finalized block exist")
            .hash()
    }

    // main logic
    async fn tick(&mut self) -> Result<(), ActorError> {
        let (_best_block, finalized_block) = self.best_and_finalized().await?;

        // update the current finalized block
        if self.curr_finalized_block_num() != finalized_block {
            // finalized block must exist
            let header = self
                .header(finalized_block)?
                .expect("finalized block must exist");
            self.queue.insert(finalized_block, header);
            // remove the finalized blocks (remain one finalized block on the front) from the queue.
            self.queue.retain(|h, _| *h >= finalized_block);
        }

        if self.curr_block + self.config.max_block_load <= finalized_block {
            // Haven't caught up with the latest finalized block
            self.tick_batch().await?;
        } else {
            if self.curr_block < finalized_block {
                // It's about to catch up with the latest finalized block
                // next_block (self.curr_block + 1) <= finalized_block
                let next_block = self.curr_block + 1;
                log::debug!(target: "actor", "BlockActor[0] Crawling Block #{}", next_block);
                self.tick_one_about_to_catch_up(next_block).await?;
            } else {
                // Has caught up with the latest finalized block
                self.tick_one_has_caught_up().await?;
            }
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

    // Haven't caught up with the latest finalized block
    //
    // curr_block                       curr_finalized_block
    //    |                                       |
    // +--+--+    +-----+    +-----+           +--+--+
    // |     |◄---+     |◄---+     |◄---...◄---+     |    queue: [curr_finalized_block]
    // +--+--+    +--+--+    +-----+           +--+--+
    //    |-------- >= max_block_load ------------|
    async fn tick_batch(&mut self) -> Result<(), ActorError> {
        // example:
        // curr_block = 0; max_block_load = 10  => (0+1..0+10] => (1..10]
        // curr_block = 10; max_block_load = 10 => (10+1..10+10] => (11..20]
        let fut = ((self.curr_block + 1)..=(self.curr_block + self.config.max_block_load))
            .map(|i| {
                let index = (i % self.config.max_block_load) as usize;
                log::debug!(target: "actor", "BlockActor[{}] Crawling Block #{}", index, i);
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

    // It's about to catch up with the latest finalized block (curr_block < finalized_block)
    //
    // curr_block                       curr_finalized_block
    //    |                                       |
    // +--+--+    +-----+    +-----+           +--+--+
    // |     |◄---+     |◄---+     |◄---...◄---+     |    queue: [curr_finalized_block]
    // +-----+    +--+--+    +-----+           +--+--+
    //    |-------- < max_block_load ------------|
    async fn tick_one_about_to_catch_up(&mut self, next_block: u32) -> Result<(), ActorError> {
        let block = self
            .crawl_next_block(next_block)
            .await?
            .expect("finalized block must exist");
        self.metadata.send(block).await?;
        self.curr_block = next_block;
        Ok(())
    }

    // Has caught up with the latest finalized block
    //
    // curr_finalized_block                   curr_block
    //    |                                       |
    // +--+--+    +-----+    +-----+           +--+--+
    // |     |◄---+     |◄---+     |◄---...◄---+     |
    // +--+--+    +--+--+    +-----+           +--+--+
    //    |                                       |
    //    +---------------------------------------+
    //               queue (length >= 1)
    async fn tick_one_has_caught_up(&mut self) -> Result<(), ActorError> {
        loop {
            // self.curr_block >= finalized_block, next_block (self.curr_block + 1) > finalized_block
            // so next block is not finalized block.
            let next_block = self.curr_block + 1;
            log::debug!(target: "actor", "BlockActor[0] Crawling Block #{}", next_block);

            if let Some(block) = self.crawl_next_block(next_block).await? {
                let next_header = block.inner.block.header();
                if next_header.parent_hash() != &self.queue_back_block_hash() {
                    // the block is forked
                    let curr_block = self.curr_block;
                    let curr_finalized_block = self.curr_finalized_block_num();

                    if curr_block > curr_finalized_block {
                        self.queue
                            .retain(|h, _| *h < curr_block && *h >= curr_finalized_block);
                        self.curr_block -= 1;
                    } else {
                        // fix issue https://github.com/patractlabs/archive/issues/62

                        // curr_block <= curr_finalized_block
                        // re-construct queue
                        let finalized_block = self
                            .crawl_next_block(curr_finalized_block)
                            .await?
                            .expect("finalized block must exist");
                        let finalized_header = finalized_block.inner.block.header();
                        self.queue.clear();
                        self.queue
                            .insert(curr_finalized_block, finalized_header.clone());
                        // reset the curr_block
                        self.curr_block = curr_finalized_block;
                    }

                    log::info!(
                        target: "actor",
                        "♻️  Rollback to Block #{}, Finalized Block #{}",
                        self.curr_block, curr_finalized_block
                    );
                    self.db
                        .send(DbDeleteGtBlockNum::new(self.curr_block))
                        .await??;
                } else {
                    // the block is valid
                    self.metadata.send(block.clone()).await?;
                    self.queue.insert(next_block, next_header.clone());
                    self.curr_block = next_block;
                    break;
                }
            } else {
                tokio::time::sleep(Duration::from_millis(self.config.interval_ms)).await;
                break;
            }
        }
        Ok(())
    }

    async fn crawl_next_block(
        &self,
        block_num: u32,
    ) -> Result<Option<BlockMessage<Block>>, ActorError> {
        let actor = self.blocks.first().expect("At least one block actor");
        Ok(actor.send(CrawlBlock::new(block_num)).await?)
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
