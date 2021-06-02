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

use crate::{
    config::{ActorConfig, DispatcherConfig},
    error::ActorError,
    message::*,
};

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
    fn spawn_dispatcher(
        config: DispatcherConfig,
    ) -> Result<dispatch::Dispatcher<Block>, ActorError> {
        let mut dispatcher = dispatch::Dispatcher::<Block>::new();
        if let Some(config) = config.kafka {
            let kafka = dispatch::kafka::KafkaActor::<Block>::new(config)?
                .create(None)
                .spawn_global();
            log::info!(target: "actor", "Spawn Kafka Actor");
            dispatcher.add("kafka", kafka);
            log::info!(target: "actor", "Add Kafka Actor into dispatcher");
        }
        Ok(dispatcher)
    }

    pub async fn spawn(
        backend: Arc<Backend>,
        api: Arc<Api>,
        config: ActorConfig,
    ) -> Result<Self, ActorError> {
        let dispatcher = Self::spawn_dispatcher(config.dispatcher)?;
        let db = postgres::PostgresActor::<Block>::new(config.postgres, dispatcher)
            .await?
            .create(None)
            .spawn_global();
        log::info!(target: "actor", "Spawn Postgres Actor");

        let metadata = metadata::MetadataActor::<Block>::new(api.clone(), db.clone())
            .create(None)
            .spawn_global();
        log::info!(target: "actor", "Spawn Metadata Actor");

        let block = block::BlockActor::<Block, Backend, Api>::new(
            backend,
            api,
            metadata.clone(),
            db.clone(),
            config.genesis,
        )
        .create(None)
        .spawn_global();
        log::info!(target: "actor", "Spawn Block Actor");

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
