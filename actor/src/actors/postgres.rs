use std::{mem, time::Duration};

use xtra::prelude::*;

use sp_runtime::traits::Block as BlockT;

use archive_postgres::{query, BlockModel, MetadataModel, PostgresConfig, PostgresDb};

use crate::{
    actors::dispatch::DispatcherActor,
    error::ActorError,
    types::{Block, CheckIfMetadataExist, Die, MaxBlock, Metadata},
};

pub struct PostgresActor<B: BlockT> {
    db: PostgresDb,
    dispatcher: Address<DispatcherActor<B>>,
}

impl<B: BlockT> PostgresActor<B> {
    pub async fn new(
        config: PostgresConfig,
        dispatcher: Address<DispatcherActor<B>>,
    ) -> Result<Self, ActorError> {
        let db = PostgresDb::new(config).await?;
        Ok(Self { db, dispatcher })
    }

    async fn metadata_handler(&self, metadata: Metadata<B>) -> Result<(), ActorError> {
        self.db
            .insert(MetadataModel::from(metadata.clone()))
            .await?;
        self.dispatcher.send(metadata).await?;
        Ok(())
    }

    async fn block_handler(&self, block: Block<B>) -> Result<(), ActorError> {
        let mut conn = self.db.conn().await?;
        while !query::check_if_metadata_exists(block.spec_version, &mut conn).await? {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        mem::drop(conn);
        self.db.insert(BlockModel::from(block.clone())).await?;
        self.dispatcher.send(block).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl<B: BlockT> Actor for PostgresActor<B> {}

#[async_trait::async_trait]
impl<B: BlockT> Handler<Metadata<B>> for PostgresActor<B> {
    async fn handle(
        &mut self,
        message: Metadata<B>,
        _ctx: &mut Context<Self>,
    ) -> <Metadata<B> as Message>::Result {
        if let Err(err) = self.metadata_handler(message).await {
            log::error!("{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<B: BlockT> Handler<Block<B>> for PostgresActor<B> {
    async fn handle(
        &mut self,
        message: Block<B>,
        _ctx: &mut Context<Self>,
    ) -> <Block<B> as Message>::Result {
        if let Err(err) = self.block_handler(message).await {
            log::error!("{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<B: BlockT> Handler<CheckIfMetadataExist> for PostgresActor<B> {
    async fn handle(
        &mut self,
        message: CheckIfMetadataExist,
        _ctx: &mut Context<Self>,
    ) -> <CheckIfMetadataExist as Message>::Result {
        match self.db.check_if_metadata_exists(message.spec_version).await {
            Ok(does_exist) => does_exist,
            Err(err) => {
                log::error!("{}", err);
                false
            }
        }
    }
}

#[async_trait::async_trait]
impl<B: BlockT> Handler<MaxBlock> for PostgresActor<B> {
    async fn handle(
        &mut self,
        _: MaxBlock,
        _: &mut Context<Self>,
    ) -> <MaxBlock as Message>::Result {
        match self.db.max_block_num().await {
            Ok(num) => num,
            Err(err) => {
                log::error!("{}", err);
                None
            }
        }
    }
}

#[async_trait::async_trait]
impl<B: BlockT> Handler<Die> for PostgresActor<B> {
    async fn handle(&mut self, message: Die, ctx: &mut Context<Self>) -> <Die as Message>::Result {
        log::info!("Stopping Postgres Actor");
        if let Err(err) = self.dispatcher.send(message).await {
            log::error!("{}", err);
        }
        ctx.stop();
    }
}
