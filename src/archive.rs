use std::{marker::PhantomData, sync::Arc, time::Instant};

use sc_client_api::backend::{Backend, StateBackendFor};
use sc_executor::NativeExecutionDispatch;
use sp_api::{ApiExt, BlockId, ConstructRuntimeApi, Core as CoreApi, Metadata as MetadataApi};
use sp_blockchain::Backend as BlockchainBackend;
use sp_runtime::{traits::Block as BlockT, BuildStorage};

use archive_actor::{ActorConfig, Actors, DispatcherConfig};
use archive_client::{
    new_archive_client, new_secondary_rocksdb_backend, ApiAccess, ArchiveBackend, ArchiveClient,
};
use archive_postgres::migrate;

use crate::{cli::ArchiveConfig, error::ArchiveError};

pub trait Archive {
    /// start driving the execution of the archive.
    fn drive(&self) -> Result<(), ArchiveError>;

    /// shutdown the archive system.
    fn shutdown(self) -> Result<(), ArchiveError>;

    /// Shutdown the archive system when self is boxed (useful when erasing the types of the runtime).
    fn boxed_shutdown(self: Box<Self>) -> Result<(), ArchiveError>;
}

pub struct ArchiveSystem<Block, Client, RA> {
    start_tx: flume::Sender<()>,
    kill_tx: flume::Sender<()>,
    handle: jod_thread::JoinHandle<Result<(), ArchiveError>>,
    _marker: PhantomData<(Block, Client, RA)>,
}

impl<Block, Client, RA> ArchiveSystem<Block, Client, RA>
where
    Block: BlockT,
    Client: ApiAccess<Block, ArchiveBackend<Block>, RA> + 'static,
    RA: ConstructRuntimeApi<Block, Client> + Send + Sync + 'static,
    <RA as ConstructRuntimeApi<Block, Client>>::RuntimeApi: CoreApi<Block>
        + MetadataApi<Block>
        + ApiExt<Block, StateBackend = StateBackendFor<ArchiveBackend<Block>, Block>>,
{
    pub fn new(
        backend: Arc<ArchiveBackend<Block>>,
        client: Arc<Client>,
        config: ActorConfig,
    ) -> Result<Self, ArchiveError> {
        let (start_tx, start_rx) = flume::bounded(1);
        let (kill_tx, kill_rx) = flume::bounded(1);

        let runtime = tokio::runtime::Runtime::new()?;
        // execute sql migrations before spawning tht actors.
        runtime.block_on(migrate(&config.postgres.uri))?;

        log::info!(target: "archive", "Start Archive Task");
        let handle = jod_thread::spawn(move || {
            start_rx.recv().expect("Start Archive Work Loop");
            log::info!(target: "archive", "Start Archive Work Loop");
            runtime.block_on(Self::work(backend, client, config, kill_rx))?;
            Ok(())
        });

        Ok(Self {
            start_tx,
            kill_tx,
            handle,
            _marker: PhantomData,
        })
    }

    fn drive(&self) -> Result<(), ArchiveError> {
        self.start_tx.send(())?;
        Ok(())
    }

    fn shutdown(self) -> Result<(), ArchiveError> {
        self.kill_tx.send(())?;
        self.handle.join()?;
        Ok(())
    }

    async fn work(
        backend: Arc<ArchiveBackend<Block>>,
        client: Arc<Client>,
        config: ActorConfig,
        kill_rx: flume::Receiver<()>,
    ) -> Result<(), ArchiveError> {
        log::info!(target: "archive", "Spawn All Actors");
        let actors = Actors::spawn(backend, client, config).await?;
        actors.tick_interval().await?;
        // waiting until the kill signal is received.
        let _ = kill_rx.recv_async().await;
        log::info!(target: "archive", "Stopping All Actors");
        actors.kill().await?;
        log::info!(target: "archive", "Stopped All Actors");
        Ok(())
    }
}

impl<Block, Client, RA> Archive for ArchiveSystem<Block, Client, RA>
where
    Block: BlockT,
    Client: ApiAccess<Block, ArchiveBackend<Block>, RA> + 'static,
    RA: ConstructRuntimeApi<Block, Client> + Send + Sync + 'static,
    <RA as ConstructRuntimeApi<Block, Client>>::RuntimeApi: CoreApi<Block>
        + MetadataApi<Block>
        + ApiExt<Block, StateBackend = StateBackendFor<ArchiveBackend<Block>, Block>>,
{
    fn drive(&self) -> Result<(), ArchiveError> {
        ArchiveSystem::drive(self)?;
        Ok(())
    }

    fn shutdown(self) -> Result<(), ArchiveError> {
        let now = Instant::now();
        ArchiveSystem::shutdown(self)?;
        log::debug!(target: "archive", "Shutdown archive took {}ms", now.elapsed().as_millis());
        Ok(())
    }

    fn boxed_shutdown(self: Box<Self>) -> Result<(), ArchiveError> {
        self.shutdown()
    }
}

pub struct ArchiveSystemBuilder<Block, Executor, RA> {
    config: ArchiveConfig,
    _marker: PhantomData<(Block, Executor, RA)>,
}

impl<Block, Executor, RA> ArchiveSystemBuilder<Block, Executor, RA>
where
    Block: BlockT,
    Executor: NativeExecutionDispatch + 'static,
    RA: ConstructRuntimeApi<Block, ArchiveClient<Block, Executor, RA>> + Send + Sync + 'static,
    <RA as ConstructRuntimeApi<Block, ArchiveClient<Block, Executor, RA>>>::RuntimeApi: CoreApi<Block>
        + MetadataApi<Block>
        + ApiExt<Block, StateBackend = StateBackendFor<ArchiveBackend<Block>, Block>>,
{
    pub fn with_config(config: ArchiveConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    pub fn build(self, genesis: &dyn BuildStorage) -> Result<impl Archive, ArchiveError> {
        let backend = new_secondary_rocksdb_backend(self.config.client.rocksdb.clone())?;
        let backend = Arc::new(backend);
        let client =
            new_archive_client::<Block, Executor, RA>(backend.clone(), self.config.client)?;
        let client = Arc::new(client);

        Self::startup_info(&*client)?;

        let system = ArchiveSystem::<_, _, RA>::new(
            backend,
            client,
            ActorConfig {
                genesis: genesis.build_storage().expect("build genesis storage"),
                postgres: self.config.postgres,
                dispatcher: DispatcherConfig {
                    kafka: self.config.kafka,
                },
            },
        )?;
        Ok(system)
    }

    /// Log some general startup info
    fn startup_info(client: &ArchiveClient<Block, Executor, RA>) -> Result<(), ArchiveError> {
        let last_finalized_block = client.backend().blockchain().last_finalized()?;
        let rt = client.runtime_version(&BlockId::Hash(last_finalized_block))?;
        log::info!(
            target: "archive",
            "Running archive for 🔗 `{}`, implementation `{}`. Latest known runtime version: {}. Latest finalized block {} 🛡️",
            rt.spec_name,
            rt.impl_name,
            rt.spec_version,
            last_finalized_block
        );
        /// The recommended open file descriptor limit to be configured for the process.
        const RECOMMENDED_OPEN_FILE_DESCRIPTOR_LIMIT: u64 = 10_000;
        if let Some(new_limit) = fdlimit::raise_fd_limit() {
            if new_limit < RECOMMENDED_OPEN_FILE_DESCRIPTOR_LIMIT {
                log::warn!(
                    target: "archive",
                    "⚠️  Low open file descriptor limit configured for the process. \
                     Current value: {:?}, recommended value: {:?}.",
                    new_limit,
                    RECOMMENDED_OPEN_FILE_DESCRIPTOR_LIMIT,
                );
            }
        }
        Ok(())
    }
}
