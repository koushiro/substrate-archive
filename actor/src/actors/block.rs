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
use sp_blockchain::HeaderBackend;
use sp_runtime::{
    traits::{Block as BlockT, Header as HeaderT, NumberFor},
    SaturatedConversion,
};
use sp_storage::Storage;
use sp_version::RuntimeVersion;

use crate::{
    actors::{metadata::MetadataActor, postgres::PostgresActor},
    error::ActorError,
    exec::BlockExecutor,
    message::{
        BestAndFinalized, BestBlockMessage, BlockMessage, Crawl, Die, FinalizedBlockMessage,
        MaxBlock, ReIndex,
    },
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
    best_block: u32,
    finalized_block: u32,
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
            best_block: 0,
            finalized_block: 0,
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

    async fn re_index(&mut self) -> Result<(), ActorError> {
        let info = self.backend.blockchain().info();
        let (best_number, finalized_number) = (
            info.best_number.saturated_into::<u32>(),
            info.finalized_number.saturated_into::<u32>(),
        );
        self.best_block = best_number;
        self.finalized_block = finalized_number;
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

            let message = BlockMessage {
                spec_version: runtime_version.spec_version,
                inner: block,
                main_changes: changes.main_storage_changes,
                child_changes: changes.child_storage_changes,
            };
            self.metadata.send(message).await?;
            self.curr_block += 1;
            Ok(())
        } else {
            // log::debug!(target: "actor", "Waiting for crawling block #{}", self.curr_block);
            tokio::time::sleep(Duration::from_millis(500)).await;
            Ok(())
        }
    }

    async fn best_and_finalized(&mut self) -> Result<(), ActorError> {
        let info = self.backend.blockchain().info();
        let (best_number, best_hash, finalized_number, finalized_hash) = (
            info.best_number,
            info.best_hash,
            info.finalized_number,
            info.finalized_hash,
        );
        if best_number.saturated_into::<u32>() > self.best_block {
            log::info!(
                target: "actor",
                "Get Best Block #{}, hash = {:?}",
                best_number, best_hash,
            );
            self.db
                .send(BestBlockMessage {
                    block_num: best_number,
                    block_hash: best_hash,
                })
                .await?;
            self.best_block = best_number.saturated_into::<u32>();
        }
        if finalized_number.saturated_into::<u32>() > self.finalized_block {
            log::info!(
                target: "actor",
                "Get Finalized Block #{}, hash = {:?}",
                finalized_number, finalized_hash,
            );
            self.db
                .send(FinalizedBlockMessage {
                    block_num: finalized_number,
                    block_hash: finalized_hash,
                })
                .await?;
            self.finalized_block = finalized_number.saturated_into::<u32>();
        }
        Ok(())
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
    async fn handle(&mut self, _: Crawl, _: &mut Context<Self>) -> <Crawl as Message>::Result {
        match self.crawl().await {
            Ok(()) => {}
            // Don't stop the actor
            Err(err) => log::error!(target: "actor", "{}", err),
        }
    }
}

#[async_trait::async_trait]
impl<Block, Backend, Api> Handler<BestAndFinalized> for BlockActor<Block, Backend, Api>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + BlockBackend<Block> + 'static,
    Api: ProvideRuntimeApi<Block> + Send + Sync + 'static,
    <Api as ProvideRuntimeApi<Block>>::Api:
        CoreApi<Block> + ApiExt<Block, StateBackend = StateBackendFor<Backend, Block>>,
{
    async fn handle(
        &mut self,
        _: BestAndFinalized,
        ctx: &mut Context<Self>,
    ) -> <BestAndFinalized as Message>::Result {
        match self.best_and_finalized().await {
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
