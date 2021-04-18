use xtra::prelude::*;

// use std::sync::Arc;

// use sp_api::{BlockId, Metadata as MetadataApi};
// use sp_core::Bytes;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};

use crate::{
    actors::postgres::PostgresActor,
    types::{Block, CheckIfMetadataExist, Die, Metadata},
    ActorError,
};

pub struct MetadataActor<B: BlockT> {
    // meta: Arc<dyn MetadataApi<B>>,
    db: Address<PostgresActor<B>>,
}

impl<B: BlockT> MetadataActor<B> {
    pub fn new(/*meta: Arc<dyn MetadataApi<B>>, */ db: Address<PostgresActor<B>>) -> Self {
        Self { db }
    }

    async fn meta_checker(
        &mut self,
        spec_version: u32,
        block_num: <B::Header as HeaderT>::Number,
        block_hash: B::Hash,
    ) -> Result<(), ActorError> {
        let is_exist = self.db.send(CheckIfMetadataExist { spec_version }).await?;
        if !is_exist {
            log::info!(
                "Getting metadata, version = {}, block_num = {}, block_hash = {}",
                spec_version,
                block_num,
                block_hash
            );
            /*
            let meta = self.meta.clone();
            let id = BlockId::Hash(block_hash);
            let meta = tokio::task::spawn_blocking(move || meta.metadata(&id)).await??;
            let metadata = Metadata {
                spec_version,
                block_num,
                block_hash,
                meta: Bytes::from(meta).0,
            };
            */
            let metadata = Metadata {
                spec_version,
                block_num,
                block_hash,
                meta: vec![],
            };
            self.db.send(metadata).await?;
        }
        Ok(())
    }

    async fn block_handler(&mut self, message: Block<B>) -> Result<(), ActorError> {
        self.meta_checker(
            message.spec_version,
            *message.inner.block.header().number(),
            message.inner.block.hash(),
        )
        .await?;
        self.db.send(message).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl<B: BlockT> Actor for MetadataActor<B> {}

#[async_trait::async_trait]
impl<B: BlockT> Handler<Block<B>> for MetadataActor<B> {
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
impl<B: BlockT> Handler<Die> for MetadataActor<B> {
    async fn handle(&mut self, _message: Die, ctx: &mut Context<Self>) -> <Die as Message>::Result {
        log::info!("Stopping Metadata Actor");
        ctx.stop();
    }
}
