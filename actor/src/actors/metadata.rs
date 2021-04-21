use xtra::prelude::*;

use std::sync::Arc;

use sp_api::{BlockId, Metadata as MetadataApi, ProvideRuntimeApi};
use sp_core::Bytes;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};

use crate::{
    actors::postgres::PostgresActor,
    error::ActorError,
    messages::{BlockMessage, CheckIfMetadataExist, Die, MetadataMessage},
};

pub struct MetadataActor<Block: BlockT, B> {
    backend: Arc<B>,
    db: Address<PostgresActor<Block>>,
}

impl<Block, B> MetadataActor<Block, B>
where
    Block: BlockT,
    B: ProvideRuntimeApi<Block> + Send + Sync + 'static,
    <B as ProvideRuntimeApi<Block>>::Api: MetadataApi<Block>,
{
    pub fn new(backend: Arc<B>, db: Address<PostgresActor<Block>>) -> Self {
        Self { backend, db }
    }

    async fn meta_checker(
        &mut self,
        spec_version: u32,
        block_num: <Block::Header as HeaderT>::Number,
        block_hash: Block::Hash,
    ) -> Result<(), ActorError> {
        let is_exist = self.db.send(CheckIfMetadataExist { spec_version }).await?;
        if !is_exist {
            log::info!(
                "Getting metadata, version = {}, block_num = {}, block_hash = {}",
                spec_version,
                block_num,
                block_hash
            );
            let backend = self.backend.clone();
            let id = BlockId::Hash(block_hash);
            let meta =
                tokio::task::spawn_blocking(move || backend.runtime_api().metadata(&id)).await??;
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

    async fn block_handler(&mut self, message: BlockMessage<Block>) -> Result<(), ActorError> {
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
impl<Block, B> Actor for MetadataActor<Block, B>
where
    Block: BlockT,
    B: Send + Sync + 'static,
{
}

#[async_trait::async_trait]
impl<Block, B> Handler<BlockMessage<Block>> for MetadataActor<Block, B>
where
    Block: BlockT,
    B: ProvideRuntimeApi<Block> + Send + Sync + 'static,
    <B as ProvideRuntimeApi<Block>>::Api: MetadataApi<Block>,
{
    async fn handle(
        &mut self,
        message: BlockMessage<Block>,
        _: &mut Context<Self>,
    ) -> <BlockMessage<Block> as Message>::Result {
        if let Err(err) = self.block_handler(message).await {
            log::error!("{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<Block, B> Handler<Die> for MetadataActor<Block, B>
where
    Block: BlockT,
    B: Send + Sync + 'static,
{
    async fn handle(&mut self, _: Die, ctx: &mut Context<Self>) -> <Die as Message>::Result {
        log::info!("Stopping Metadata Actor");
        ctx.stop();
    }
}
