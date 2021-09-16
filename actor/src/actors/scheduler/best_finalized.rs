use std::{sync::Arc, time::Duration};

use chrono::Utc;
use xtra::prelude::*;

use sc_client_api::backend;
use sp_blockchain::HeaderBackend;
use sp_runtime::{traits::Block as BlockT, SaturatedConversion};

use crate::{
    actors::postgres::PostgresActor,
    error::ActorError,
    message::{
        BestAndFinalized, BestBlockMessage, CrawlBestAndFinalized, DbBestBlock, DbFinalizedBlock,
        Die, FinalizedBlockMessage,
    },
};

pub struct BestAndFinalizedActor<Block: BlockT, Backend> {
    backend: Arc<Backend>,
    db: Address<PostgresActor<Block>>,
    interval_ms: u64,
    best_block_num: u32,
    finalized_block_num: u32,
}

impl<Block, Backend> BestAndFinalizedActor<Block, Backend>
where
    Block: BlockT,
    Backend: backend::Backend<Block>,
{
    pub fn new(backend: Arc<Backend>, db: Address<PostgresActor<Block>>, interval_ms: u64) -> Self {
        Self {
            backend,
            db,
            interval_ms,
            best_block_num: 0,
            finalized_block_num: 0,
        }
    }

    async fn initialize(&mut self) -> Result<(), ActorError> {
        let (best_block, best_block_hash) = self.db.send(DbBestBlock).await??.unwrap_or_default();
        self.best_block_num = best_block;

        let (finalized_block, finalized_block_hash) =
            self.db.send(DbFinalizedBlock).await??.unwrap_or_default();
        self.finalized_block_num = finalized_block;

        log::info!(
            target: "actor",
            "Initialized BestAndFinalized Actor. \
            Best Block #{} (0x{}), Finalized Block #{} (0x{})",
            self.best_block_num, hex::encode(best_block_hash),
            self.finalized_block_num, hex::encode(finalized_block_hash)
        );
        Ok(())
    }

    async fn crawl(&mut self) -> Result<(), ActorError> {
        let info = self.backend.blockchain().info();
        let (best_number, best_hash, finalized_number, finalized_hash) = (
            info.best_number,
            info.best_hash,
            info.finalized_number,
            info.finalized_hash,
        );
        if best_number.saturated_into::<u32>() > self.best_block_num {
            log::info!(
                target: "actor",
                "Crawl Best Block #{} ({:?})",
                best_number, best_hash,
            );
            self.db
                .send(BestBlockMessage {
                    block_num: best_number,
                    block_hash: best_hash,
                })
                .await?;
            self.best_block_num = best_number.saturated_into::<u32>();
        }
        if finalized_number.saturated_into::<u32>() > self.finalized_block_num {
            log::info!(
                target: "actor",
                "Crawl Finalized Block #{} ({:?})",
                finalized_number, finalized_hash,
            );
            self.db
                .send(FinalizedBlockMessage {
                    block_num: finalized_number,
                    block_hash: finalized_hash,
                    timestamp: Utc::now().timestamp_millis(),
                })
                .await?;
            self.finalized_block_num = finalized_number.saturated_into::<u32>();
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl<Block, Backend> Actor for BestAndFinalizedActor<Block, Backend>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + 'static,
{
    async fn started(&mut self, ctx: &mut Context<Self>) {
        self.initialize()
            .await
            .expect("Initialization should not fail");
        let addr = ctx.address().expect("Actor just started");
        let interval_ms = self.interval_ms;
        tokio::task::spawn(async move {
            loop {
                if addr.send(CrawlBestAndFinalized).await.is_err() {
                    log::error!(target: "actor", "BestAndFinalized Actor Disconnected");
                    break;
                }
                tokio::time::sleep(Duration::from_millis(interval_ms)).await;
            }
        });
    }
}

#[async_trait::async_trait]
impl<Block, Backend> Handler<CrawlBestAndFinalized> for BestAndFinalizedActor<Block, Backend>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + 'static,
{
    async fn handle(
        &mut self,
        _: CrawlBestAndFinalized,
        _: &mut Context<Self>,
    ) -> <CrawlBestAndFinalized as Message>::Result {
        match self.crawl().await {
            Ok(()) => {}
            // if error occurred, we don't stop the actor.
            Err(err) => log::error!(target: "actor", "Crawl Best And Finalized Block: {}", err),
        }
    }
}

#[async_trait::async_trait]
impl<Block, Backend> Handler<BestAndFinalized> for BestAndFinalizedActor<Block, Backend>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + 'static,
{
    async fn handle(
        &mut self,
        _: BestAndFinalized,
        _: &mut Context<Self>,
    ) -> <BestAndFinalized as Message>::Result {
        (self.best_block_num, self.finalized_block_num)
    }
}

#[async_trait::async_trait]
impl<Block, Backend> Handler<Die> for BestAndFinalizedActor<Block, Backend>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + 'static,
{
    async fn handle(&mut self, _: Die, ctx: &mut Context<Self>) -> <Die as Message>::Result {
        log::info!(target: "actor", "Stopping BestAndFinalized Actor");
        ctx.stop();
    }
}
