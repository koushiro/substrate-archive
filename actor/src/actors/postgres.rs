use xtra::prelude::*;

use sp_runtime::traits::Block as BlockT;

use archive_postgres::{BlockModel, MetadataModel, PostgresConfig, PostgresDb, SqlxError};

use crate::types::{Block, CheckIfMetadataExist, Die, Metadata};

pub struct PostgresActor<B: BlockT> {
    db: PostgresDb,
    _marker: std::marker::PhantomData<B>,
}

impl<B: BlockT> PostgresActor<B> {
    pub async fn new(config: PostgresConfig) -> Result<Self, SqlxError> {
        let db = PostgresDb::new(config).await?;
        Ok(Self {
            db,
            _marker: std::marker::PhantomData,
        })
    }

    async fn metadata_handler(&self, metadata: Metadata<B>) -> Result<(), SqlxError> {
        let model = MetadataModel::from(metadata);
        self.db.insert(model).await?;
        Ok(())
    }

    async fn block_handler(&self, block: Block<B>) -> Result<(), SqlxError> {
        let model = BlockModel::from(block);
        self.db.insert(model).await?;
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
impl<B: BlockT> Handler<Die> for PostgresActor<B> {
    async fn handle(&mut self, _message: Die, ctx: &mut Context<Self>) -> <Die as Message>::Result {
        log::info!("Stopping Postgres Actor");
        ctx.stop();
    }
}
