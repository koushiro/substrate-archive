// mod cache;
mod config;
mod types;
// mod workers;

/*
use futures::future::BoxFuture;
use xtra::{prelude::*, spawn::TokioGlobalSpawnExt, Disconnected};

use sp_runtime::traits::Block as BlockT;

use self::workers::*;
use crate::actors::types::Die;

pub struct Actors<B: BlockT> {
    db: Address<postgres::PostgresActor<B>>,
    metadata: Address<metadata::MetadataActor<B>>,
    block: Address<block::BlockActor<B>>,
}

impl<B: BlockT> Actors<B> {
    pub fn spawn() -> Self {
        let db = postgres::PostgresActor::new().create(None).spawn_global();
        let metadata = metadata::MetadataActor::new().create(None).spawn_global();
        let block = block::BlockActor::new().create(None).spawn_global();
        Self {
            db,
            metadata,
            block,
        }
    }

    pub async fn kill(self) -> Result<(), Disconnected> {
        type SendDieResult = Result<<Die as Message>::Result, Disconnected>;
        let fut: Vec<BoxFuture<'_, SendDieResult>> = vec![
            Box::pin(self.block.send(Die)),
            Box::pin(self.metadata.send(Die)),
            Box::pin(self.db.send(Die)),
        ];
        let results = futures::future::join_all(fut).await;
        results.into_iter().try_for_each(|result| result)?;
        Ok(())
    }
}
*/
