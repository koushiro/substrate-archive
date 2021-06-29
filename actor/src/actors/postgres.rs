use std::{mem, time::Duration};

use xtra::prelude::*;

use sp_runtime::traits::Block as BlockT;

use archive_postgres::{model::*, query, PostgresConfig, PostgresDb};

use crate::{
    actors::dispatch::Dispatcher,
    error::ActorError,
    message::{
        BestBlock, BestBlockMessage, BlockMessage, CheckIfMetadataExist, Die, FinalizedBlock,
        FinalizedBlockMessage, MaxBlock, MetadataMessage,
    },
};

pub struct PostgresActor<Block: BlockT> {
    db: PostgresDb,
    dispatcher: Dispatcher<Block>,
}

impl<Block: BlockT> PostgresActor<Block> {
    pub async fn new(
        config: PostgresConfig,
        dispatcher: Dispatcher<Block>,
    ) -> Result<Self, ActorError> {
        let db = PostgresDb::new(config).await?;
        Ok(Self { db, dispatcher })
    }

    async fn metadata_handler(&self, metadata: MetadataMessage<Block>) -> Result<(), ActorError> {
        self.db
            .insert(MetadataModel::from(metadata.clone()))
            .await?;
        self.dispatcher.dispatch_metadata(metadata).await?;
        Ok(())
    }

    async fn block_handler(&self, message: BlockMessage<Block>) -> Result<(), ActorError> {
        let mut conn = self.db.conn().await?;
        while !query::check_if_metadata_exists(message.spec_version, &mut conn).await? {
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
        self.dispatcher.dispatch_block(message).await?;
        Ok(())
    }

    async fn best_block_handler(&self, message: BestBlockMessage<Block>) -> Result<(), ActorError> {
        self.db
            .insert(BestBlockModel::from(message.clone()))
            .await?;
        self.dispatcher.dispatch_best_block(message).await?;
        Ok(())
    }

    async fn finalized_block_handler(
        &self,
        message: FinalizedBlockMessage<Block>,
    ) -> Result<(), ActorError> {
        self.db
            .insert(FinalizedBlockModel::from(message.clone()))
            .await?;
        self.dispatcher.dispatch_finalized_block(message).await?;
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
impl<Block: BlockT> Handler<CheckIfMetadataExist> for PostgresActor<Block> {
    async fn handle(
        &mut self,
        message: CheckIfMetadataExist,
        _ctx: &mut Context<Self>,
    ) -> <CheckIfMetadataExist as Message>::Result {
        match self.db.check_if_metadata_exists(message.spec_version).await {
            Ok(does_exist) => does_exist,
            Err(err) => {
                log::error!(target: "actor", "{}", err);
                false
            }
        }
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<MaxBlock> for PostgresActor<Block> {
    async fn handle(
        &mut self,
        _: MaxBlock,
        _: &mut Context<Self>,
    ) -> <MaxBlock as Message>::Result {
        match self.db.max_block_num().await {
            Ok(num) => num,
            Err(err) => {
                log::error!(target: "actor", "{}", err);
                None
            }
        }
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<BestBlock> for PostgresActor<Block> {
    async fn handle(
        &mut self,
        _: BestBlock,
        _: &mut Context<Self>,
    ) -> <BestBlock as Message>::Result {
        match self.db.max_block_num().await {
            Ok(num) => num,
            Err(err) => {
                log::error!(target: "actor", "{}", err);
                None
            }
        }
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<FinalizedBlock> for PostgresActor<Block> {
    async fn handle(
        &mut self,
        _: FinalizedBlock,
        _: &mut Context<Self>,
    ) -> <FinalizedBlock as Message>::Result {
        match self.db.max_block_num().await {
            Ok(num) => num,
            Err(err) => {
                log::error!(target: "actor", "{}", err);
                None
            }
        }
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<Die> for PostgresActor<Block> {
    async fn handle(&mut self, message: Die, ctx: &mut Context<Self>) -> <Die as Message>::Result {
        log::info!(target: "actor", "Stopping Postgres Actor");
        if let Err(err) = self.dispatcher.dispatch_die(message).await {
            log::error!(target: "actor", "{}", err);
        }
        ctx.stop();
    }
}
