use std::{collections::HashSet, mem, time::Duration};

use xtra::prelude::*;

use sp_runtime::traits::Block as BlockT;

use archive_postgres::{model::*, query, PostgresConfig, PostgresDb};

use crate::{
    actors::dispatcher::Dispatcher,
    error::ActorError,
    message::{
        BatchBlockMessage, BestBlockMessage, BlockMessage, CatchupFinalized, DbBestBlock,
        DbDeleteGtBlockNum, DbFinalizedBlock, DbIfMetadataExist, DbMaxBlock, Die,
        FinalizedBlockMessage, MetadataMessage,
    },
};

pub struct PostgresActor<Block: BlockT> {
    db: PostgresDb,
    dispatcher: Option<Dispatcher<Block>>,
    // Means if the current block of scheduler catching up the finalized block.
    catchup_finalized: bool,
}

impl<Block: BlockT> PostgresActor<Block> {
    pub async fn new(
        config: PostgresConfig,
        dispatcher: Option<Dispatcher<Block>>,
    ) -> Result<Self, ActorError> {
        let db = PostgresDb::new(config).await?;
        Ok(Self {
            db,
            dispatcher,
            catchup_finalized: false,
        })
    }

    async fn metadata_handler(&self, metadata: MetadataMessage<Block>) -> Result<(), ActorError> {
        self.db
            .insert(MetadataModel::from(metadata.clone()))
            .await?;
        if let Some(dispatcher) = &self.dispatcher {
            dispatcher.dispatch_metadata(metadata).await?;
        }
        Ok(())
    }

    async fn block_handler(&self, message: BlockMessage<Block>) -> Result<(), ActorError> {
        let mut conn = self.db.conn().await?;
        while !query::check_if_metadata_exists(message.version, &mut conn).await? {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        mem::drop(conn);

        let (block, main_storage, _child_storage): (
            BlockModel,
            Vec<MainStorageChangeModel>,
            Vec<ChildStorageChangeModel>,
        ) = message.clone().into();
        self.db.insert(block).await?;
        self.db.insert(main_storage).await?;
        // TODO: insert child storage into database.
        // self.db.insert(_child_storage).await?;
        if let Some(dispatcher) = &self.dispatcher {
            dispatcher.dispatch_block(message).await?;
        }
        Ok(())
    }

    // Returns true if all metadata versions are in database
    // Otherwise, returns false if some metadata versions are missing.
    fn db_contains_metadata(blocks: &[BlockMessage<Block>], all_versions: &HashSet<u32>) -> bool {
        let versions: HashSet<u32> = blocks.iter().map(|block| block.version).collect();
        versions.is_subset(all_versions)
    }

    async fn batch_block_handler(
        &self,
        message: BatchBlockMessage<Block>,
    ) -> Result<(), ActorError> {
        let mut conn = self.db.conn().await?;
        let all_versions: HashSet<u32> = query::get_all_metadata_versions(&mut conn)
            .await?
            .into_iter()
            .collect();
        while !Self::db_contains_metadata(message.inner(), &all_versions) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        mem::drop(conn);

        let (blocks, main_storages, _child_storages): (
            Vec<BlockModel>,
            Vec<MainStorageChangeModel>,
            Vec<ChildStorageChangeModel>,
        ) = message.clone().into();
        self.db.insert(blocks).await?;
        self.db.insert(main_storages).await?;
        // TODO: insert child storage into database.
        // self.db.insert(_child_storages).await?;
        if let Some(dispatcher) = &self.dispatcher {
            dispatcher.dispatch_batch_block(message).await?;
        }
        Ok(())
    }

    async fn best_block_handler(&self, message: BestBlockMessage<Block>) -> Result<(), ActorError> {
        self.db
            .insert(BestBlockModel::from(message.clone()))
            .await?;
        Ok(())
    }

    async fn finalized_block_handler(
        &self,
        message: FinalizedBlockMessage<Block>,
    ) -> Result<(), ActorError> {
        self.db
            .insert(FinalizedBlockModel::from(message.clone()))
            .await?;
        if self.catchup_finalized {
            if let Some(dispatcher) = &self.dispatcher {
                dispatcher.dispatch_finalized_block(message).await?;
            }
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Actor for PostgresActor<Block> {}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<MetadataMessage<Block>> for PostgresActor<Block> {
    async fn handle(
        &mut self,
        message: MetadataMessage<Block>,
        _ctx: &mut Context<Self>,
    ) -> <MetadataMessage<Block> as Message>::Result {
        if let Err(err) = self.metadata_handler(message).await {
            log::error!(target: "actor", "{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<BlockMessage<Block>> for PostgresActor<Block> {
    async fn handle(
        &mut self,
        message: BlockMessage<Block>,
        _ctx: &mut Context<Self>,
    ) -> <BlockMessage<Block> as Message>::Result {
        if let Err(err) = self.block_handler(message).await {
            log::error!(target: "actor", "{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<BatchBlockMessage<Block>> for PostgresActor<Block> {
    async fn handle(
        &mut self,
        message: BatchBlockMessage<Block>,
        _ctx: &mut Context<Self>,
    ) -> <BatchBlockMessage<Block> as Message>::Result {
        if let Err(err) = self.batch_block_handler(message).await {
            log::error!(target: "actor", "{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<BestBlockMessage<Block>> for PostgresActor<Block> {
    async fn handle(
        &mut self,
        message: BestBlockMessage<Block>,
        _ctx: &mut Context<Self>,
    ) -> <BestBlockMessage<Block> as Message>::Result {
        if let Err(err) = self.best_block_handler(message).await {
            log::error!(target: "actor", "{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<FinalizedBlockMessage<Block>> for PostgresActor<Block> {
    async fn handle(
        &mut self,
        message: FinalizedBlockMessage<Block>,
        _ctx: &mut Context<Self>,
    ) -> <FinalizedBlockMessage<Block> as Message>::Result {
        if let Err(err) = self.finalized_block_handler(message).await {
            log::error!(target: "actor", "{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<DbIfMetadataExist> for PostgresActor<Block> {
    async fn handle(
        &mut self,
        message: DbIfMetadataExist,
        _ctx: &mut Context<Self>,
    ) -> <DbIfMetadataExist as Message>::Result {
        self.db.if_metadata_exists(message.version).await
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<DbMaxBlock> for PostgresActor<Block> {
    async fn handle(
        &mut self,
        _: DbMaxBlock,
        _: &mut Context<Self>,
    ) -> <DbMaxBlock as Message>::Result {
        self.db.max_block_num().await
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<DbBestBlock> for PostgresActor<Block> {
    async fn handle(
        &mut self,
        _: DbBestBlock,
        _: &mut Context<Self>,
    ) -> <DbBestBlock as Message>::Result {
        self.db.best_block_num().await
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<DbFinalizedBlock> for PostgresActor<Block> {
    async fn handle(
        &mut self,
        _: DbFinalizedBlock,
        _: &mut Context<Self>,
    ) -> <DbFinalizedBlock as Message>::Result {
        self.db.finalized_block_num().await
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<DbDeleteGtBlockNum> for PostgresActor<Block> {
    async fn handle(
        &mut self,
        message: DbDeleteGtBlockNum,
        _: &mut Context<Self>,
    ) -> <DbDeleteGtBlockNum as Message>::Result {
        self.db.delete(message.block_num).await
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<CatchupFinalized> for PostgresActor<Block> {
    async fn handle(
        &mut self,
        _: CatchupFinalized,
        _: &mut Context<Self>,
    ) -> <CatchupFinalized as Message>::Result {
        self.catchup_finalized = true;
        log::info!(target: "actor", "Postgres receive the CatchupFinalized");
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<Die> for PostgresActor<Block> {
    async fn handle(&mut self, message: Die, ctx: &mut Context<Self>) -> <Die as Message>::Result {
        log::info!(target: "actor", "Stopping Postgres Actor");
        if let Some(dispatcher) = &self.dispatcher {
            if let Err(err) = dispatcher.dispatch_die(message).await {
                log::error!(target: "actor", "{}", err);
            }
        }
        ctx.stop();
    }
}
