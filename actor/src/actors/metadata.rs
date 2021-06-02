use xtra::prelude::*;

use std::sync::Arc;

use sp_api::{ApiError, BlockId, Metadata as MetadataApi, ProvideRuntimeApi};
use sp_core::{Bytes, OpaqueMetadata};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};

use crate::{
    actors::postgres::PostgresActor,
    error::ActorError,
    message::{BlockMessage, CheckIfMetadataExist, Die, MetadataMessage},
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

pub struct MetadataActor<Block: BlockT> {
    api: Arc<dyn GetMetadata<Block>>,
    db: Address<PostgresActor<Block>>,
}

impl<Block> MetadataActor<Block>
where
    Block: BlockT,
{
    pub fn new(api: Arc<dyn GetMetadata<Block>>, db: Address<PostgresActor<Block>>) -> Self {
        Self { api, db }
    }

    async fn meta_checker(
        &self,
        spec_version: u32,
        block_num: <Block::Header as HeaderT>::Number,
        block_hash: Block::Hash,
    ) -> Result<(), ActorError> {
        let is_exist = self.db.send(CheckIfMetadataExist { spec_version }).await?;
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
                meta: Bytes::from(meta).0,
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
}

#[async_trait::async_trait]
impl<Block> Actor for MetadataActor<Block> where Block: BlockT {}

#[async_trait::async_trait]
impl<Block> Handler<BlockMessage<Block>> for MetadataActor<Block>
where
    Block: BlockT,
{
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
impl<Block> Handler<Die> for MetadataActor<Block>
where
    Block: BlockT,
{
    async fn handle(&mut self, _: Die, ctx: &mut Context<Self>) -> <Die as Message>::Result {
        log::info!(target: "actor", "Stopping Metadata Actor");
        ctx.stop();
    }
}
