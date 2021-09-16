use std::sync::Arc;

use itertools::Itertools;
use xtra::prelude::*;

use sp_api::{ApiError, BlockId, Metadata as MetadataApi, ProvideRuntimeApi};
use sp_core::OpaqueMetadata;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};

use crate::{
    actors::postgres::PostgresActor,
    error::ActorError,
    message::{BatchBlockMessage, BlockMessage, DbIfMetadataExist, Die, MetadataMessage},
};

pub trait GetMetadata<Block: BlockT>: Send + Sync {
    fn metadata(&self, id: &BlockId<Block>) -> Result<OpaqueMetadata, ApiError>;
}

impl<Block, Api> GetMetadata<Block> for Api
where
    Block: BlockT,
    Api: ProvideRuntimeApi<Block> + Send + Sync,
    <Api as ProvideRuntimeApi<Block>>::Api: MetadataApi<Block>,
{
    fn metadata(&self, id: &BlockId<Block>) -> Result<OpaqueMetadata, ApiError> {
        self.runtime_api().metadata(id)
    }
}

type MetaApi<Block> = Arc<dyn GetMetadata<Block>>;

pub struct MetadataActor<Block: BlockT> {
    api: MetaApi<Block>,
    db: Address<PostgresActor<Block>>,
}

impl<Block: BlockT> MetadataActor<Block> {
    pub fn new(api: MetaApi<Block>, db: Address<PostgresActor<Block>>) -> Self {
        Self { api, db }
    }

    async fn meta_checker(
        &self,
        spec_version: u32,
        block_num: <Block::Header as HeaderT>::Number,
        block_hash: Block::Hash,
    ) -> Result<(), ActorError> {
        let is_exist = self.db.send(DbIfMetadataExist { spec_version }).await??;
        if !is_exist {
            log::info!(
                target: "actor",
                "Getting metadata, version = {}, block_num = {}, block_hash = {}",
                spec_version,
                block_num,
                block_hash
            );
            let api = self.api.clone();
            let id = BlockId::Hash(block_hash);
            let meta = tokio::task::spawn_blocking(move || api.metadata(&id)).await??;
            let metadata = MetadataMessage {
                spec_version,
                block_num,
                block_hash,
                meta: meta.to_vec(),
            };
            self.db.send(metadata).await?;
        }
        Ok(())
    }

    async fn block_handler(&self, message: BlockMessage<Block>) -> Result<(), ActorError> {
        self.meta_checker(
            message.spec_version,
            *message.inner.block.header().number(),
            message.inner.block.hash(),
        )
        .await?;
        self.db.send(message).await?;
        Ok(())
    }

    async fn batch_block_handler(
        &self,
        message: BatchBlockMessage<Block>,
    ) -> Result<(), ActorError> {
        for block in message
            .inner()
            .iter()
            .unique_by(|&block| block.spec_version)
        {
            self.meta_checker(
                block.spec_version,
                *block.inner.block.header().number(),
                block.inner.block.hash(),
            )
            .await?;
        }
        self.db.send(message).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Actor for MetadataActor<Block> {}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<BlockMessage<Block>> for MetadataActor<Block> {
    async fn handle(
        &mut self,
        message: BlockMessage<Block>,
        _: &mut Context<Self>,
    ) -> <BlockMessage<Block> as Message>::Result {
        if let Err(err) = self.block_handler(message).await {
            log::error!(target: "actor", "{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<BatchBlockMessage<Block>> for MetadataActor<Block> {
    async fn handle(
        &mut self,
        message: BatchBlockMessage<Block>,
        _: &mut Context<Self>,
    ) -> <BatchBlockMessage<Block> as Message>::Result {
        if let Err(err) = self.batch_block_handler(message).await {
            log::error!(target: "actor", "{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<Die> for MetadataActor<Block> {
    async fn handle(&mut self, _: Die, ctx: &mut Context<Self>) -> <Die as Message>::Result {
        log::info!(target: "actor", "Stopping Metadata Actor");
        ctx.stop();
    }
}
