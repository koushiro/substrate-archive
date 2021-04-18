use xtra::prelude::*;

use sp_runtime::traits::Block as BlockT;

use crate::{
    actors::{metadata::MetadataActor, postgres::PostgresActor},
    types::Die,
};

#[allow(dead_code)]
pub struct BlockActor<B: BlockT> {
    metadata: Address<MetadataActor<B>>,
    db: Address<PostgresActor<B>>,
}

impl<B: BlockT> BlockActor<B> {
    pub fn new(metadata: Address<MetadataActor<B>>, db: Address<PostgresActor<B>>) -> Self {
        Self { metadata, db }
    }
}

#[async_trait::async_trait]
impl<B: BlockT> Actor for BlockActor<B> {
    async fn started(&mut self, _ctx: &mut Context<Self>) {
        todo!()
    }
}

#[async_trait::async_trait]
impl<B: BlockT> Handler<Die> for BlockActor<B> {
    async fn handle(&mut self, _message: Die, ctx: &mut Context<Self>) -> <Die as Message>::Result {
        log::info!("Stopping Block Actor");
        ctx.stop();
    }
}
