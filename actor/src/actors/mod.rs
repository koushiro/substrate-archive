mod dispatcher;
mod metadata;
mod postgres;
mod scheduler;

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
    scheduler: Address<scheduler::Scheduler<Block, Backend, Api>>,
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

        let scheduler = scheduler::Scheduler::<Block, Backend, Api>::new(
            config.genesis,
            backend,
            api,
            db.clone(),
            metadata.clone(),
            config.max_block_load,
            config.interval_ms,
        )
        .create(None)
        .spawn_global();
        log::info!(target: "actor", "Spawn Scheduler Actor");

        Ok(Self {
            db,
            metadata,
            scheduler,
        })
    }

    fn spawn_dispatcher(
        config: Option<DispatcherConfig>,
    ) -> Result<Option<dispatcher::Dispatcher<Block>>, ActorError> {
        if let Some(config) = config {
            let mut dispatcher = dispatcher::Dispatcher::<Block>::new();
            if let Some(config) = config.kafka {
                let kafka = dispatcher::kafka::KafkaActor::<Block>::new(config)?
                    .create(None)
                    .spawn_global();
                log::info!(target: "actor", "Spawn Kafka Actor");
                dispatcher.add("kafka", kafka);
                log::info!(target: "actor", "Add Kafka Actor into dispatcher");
            }
            Ok(Some(dispatcher))
        } else {
            Ok(None)
        }
    }

    pub async fn tick_interval(&self) -> Result<(), ActorError> {
        // messages that only need to be sent once
        self.scheduler.send(Initialize).await?;
        let scheduler = self.scheduler.clone();
        tokio::task::spawn(async move {
            loop {
                match scheduler.send(Tick).await {
                    Ok(Ok(_)) => {}
                    Ok(Err(err)) => {
                        log::error!(target: "actor", "Scheduler tick error: {}", err);
                        break;
                    }
                    Err(_) => {
                        log::error!(target: "actor", "Scheduler Actor Disconnected");
                        break;
                    }
                }
            }
        });
        Ok(())
    }

    pub async fn kill(self) -> Result<(), ActorError> {
        self.scheduler.send(Die).await?;
        log::info!(target: "actor", "Stopped Scheduler Actor");
        self.metadata.send(Die).await?;
        log::info!(target: "actor", "Stopped Metadata Actor");
        self.db.send(Die).await?;
        log::info!(target: "actor", "Stopped Postgres Actor");
        Ok(())
    }
}
