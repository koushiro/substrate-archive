mod block;
mod dispatch;
mod metadata;
mod postgres;

use std::sync::Arc;

use xtra::{prelude::*, spawn::TokioGlobalSpawnExt};

use sc_client_api::{
    backend::{self, StateBackendFor},
    client::BlockBackend,
};
use sp_api::{ApiExt, Core as CoreApi, Metadata as MetadataApi, ProvideRuntimeApi};
use sp_runtime::traits::Block as BlockT;

use crate::{config::ActorConfig, error::ActorError, messages::*};

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
pub struct Actors<Block, Backend, Api>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + BlockBackend<Block> + 'static,
    Api: ProvideRuntimeApi<Block> + Send + Sync + 'static,
    <Api as ProvideRuntimeApi<Block>>::Api: CoreApi<Block>
        + MetadataApi<Block>
        + ApiExt<Block, StateBackend = StateBackendFor<Backend, Block>>,
{
    db: Address<postgres::PostgresActor<Block>>,
    metadata: Address<metadata::MetadataActor<Block>>,
    block: Address<block::BlockActor<Block, Backend, Api>>,
}

impl<Block, Backend, Api> Actors<Block, Backend, Api>
where
    Block: BlockT,
    Backend: backend::Backend<Block> + BlockBackend<Block> + 'static,
    Api: ProvideRuntimeApi<Block> + Send + Sync + 'static,
    <Api as ProvideRuntimeApi<Block>>::Api: CoreApi<Block>
        + MetadataApi<Block>
        + ApiExt<Block, StateBackend = StateBackendFor<Backend, Block>>,
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

    pub async fn spawn(
        backend: Arc<Backend>,
        api: Arc<Api>,
        config: ActorConfig,
    ) -> Result<Self, ActorError> {
        let db = Self::spawn_db(config).await?;

        let metadata = metadata::MetadataActor::new(api.clone(), db.clone())
            .create(None)
            .spawn_global();
        let block = block::BlockActor::new(backend, api, metadata.clone(), db.clone())
            .create(None)
            .spawn_global();

        Ok(Self {
            db,
            metadata,
            block,
        })
    }

    pub async fn kill(self) -> Result<(), ActorError> {
        self.block.send(Die).await?;
        self.metadata.send(Die).await?;
        self.db.send(Die).await?;
        Ok(())
    }
}
