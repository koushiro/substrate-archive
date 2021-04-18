mod block;
mod dispatch;
mod metadata;
mod postgres;

use std::sync::Arc;

use xtra::{prelude::*, spawn::TokioGlobalSpawnExt, Disconnected};

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
    async fn spawn_db(
        config: ActorConfig,
    ) -> Result<Address<postgres::PostgresActor<Block>>, ActorError> {
        let mut dispatcher = dispatch::DispatcherActor::new();
        if let Some(config) = config.dispatcher.kafka {
            dispatcher = dispatcher.add("kafka", dispatch::kafka::KafkaActor::new(config)?);
        }
        let dispatcher = dispatcher.create(None).spawn_global();
        let db = postgres::PostgresActor::new(config.postgres, dispatcher)
            .await?
            .create(None)
            .spawn_global();
        Ok(db)
    }

    pub async fn spawn<Executor, RA>(
        _client: Arc<ArchiveClient<Block, Executor, RA>>,
        config: ActorConfig,
    ) -> Result<Self, ActorError> {
        let db = Self::spawn_db(config).await?;
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
