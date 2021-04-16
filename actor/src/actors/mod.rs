mod block;
mod dispatch;
mod metadata;
mod postgres;

use std::sync::Arc;

use xtra::{prelude::*, spawn::TokioGlobalSpawnExt, Disconnected};

// use sc_executor::NativeExecutionDispatch;
// use sp_api::{Metadata as MetadataApi, ProvideRuntimeApi};
use sp_runtime::traits::Block as BlockT;

use archive_client::ArchiveClient;

use crate::{config::ActorConfig, error::ActorError, types::*};

/// The direction of data flow:

///                                                     ┌───────┐
///     ┌───────────────────────────────┐         ┌────►│ kafka │
///     │                               │         │     └───────┘
/// ┌───┴───┐     ┌──────────┐     ┌────▼─────┐   │     ┌───────┐
/// │ block ├────►│ metadata ├────►│ postgres ├───┼────►│  ...  │
/// └───────┘     └──────────┘     └──────────┘   │     └───────┘
///                                               │     ┌───────┐
///                                               └────►│  ...  │
///                                                     └───────┘
///
pub struct Actors<Block: BlockT> {
    db: Address<postgres::PostgresActor<Block>>,
    metadata: Address<metadata::MetadataActor<Block>>,
    block: Address<block::BlockActor<Block>>,
}

impl<Block: BlockT> Actors<Block>
where
    Block: BlockT,
{
    pub async fn spawn<Executor, RA>(
        _client: Arc<ArchiveClient<Block, Executor, RA>>,
        config: ActorConfig,
    ) -> Result<Self, ActorError> {
        let db = postgres::PostgresActor::new(config.postgres)
            .await?
            .create(None)
            .spawn_global();
        // let meta = Arc::new(client.runtime_api());
        let metadata = metadata::MetadataActor::new(db.clone())
            .create(None)
            .spawn_global();
        let block = block::BlockActor::new(metadata.clone(), db.clone())
            .create(None)
            .spawn_global();

        Ok(Self {
            db,
            metadata,
            block,
        })
    }

    pub async fn kill(self) -> Result<(), Disconnected> {
        self.block.send(Die).await?;
        self.metadata.send(Die).await?;
        self.db.send(Die).await?;
        Ok(())
    }
}
