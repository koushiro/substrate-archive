use std::{marker::PhantomData, sync::Arc};

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
    type Error;

    fn drive(&self) -> Result<(), Self::Error>;

    fn shutdown(self) -> Result<(), Self::Error>;

    fn boxed_shutdown(self: Box<Self>) -> Result<(), Self::Error>;
}

pub struct ArchiveSystem<Block, Client, RA>
where
    Block: BlockT,
{
    backend: Arc<ArchiveBackend<Block>>,
    start_tx: flume::Sender<()>,
    kill_tx: flume::Sender<()>,
    handle: jod_thread::JoinHandle<Result<(), ArchiveError>>,
    _marker: PhantomData<(Client, RA)>,
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

        let runtime = tokio::runtime::Runtime::new().expect("cannot create async runtime");

        // execute sql migrations before spawning tht actors.
        runtime.block_on(migrate(&config.postgres.uri))?;

        let backend_clone = backend.clone();
        let handle = jod_thread::spawn(move || {
            let _ = start_rx.recv();
            log::info!(target: "archive", "Start archive main loop");
            runtime.block_on(Self::main_loop(backend_clone, client, config, kill_rx))?;
            Ok(())
        });

        Ok(Self {
            backend,
            start_tx,
            kill_tx,
            handle,
            _marker: PhantomData,
        })
    }

    async fn main_loop(
        backend: Arc<ArchiveBackend<Block>>,
        client: Arc<Client>,
        config: ActorConfig,
        kill_rx: flume::Receiver<()>,
    ) -> Result<(), ArchiveError> {
        log::info!(target: "archive", "Spawn all actors");
        let actors = Actors::spawn(backend, client, config).await?;
        // waiting until the kill signal is received.
        let _ = kill_rx.recv_async().await;
        log::info!(target: "archive", "Stop all actors");
        actors.kill().await?;
        Ok(())
    }

    pub fn backend(&self) -> Arc<ArchiveBackend<Block>> {
        self.backend.clone()
    }

    pub fn drive(&self) -> Result<(), ArchiveError> {
        self.start_tx.send(())?;
        Ok(())
    }

    pub fn shutdown(self) -> Result<(), ArchiveError> {
        self.kill_tx.send(())?;
        self.handle.join()?;
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
    type Error = ArchiveError;

    fn drive(&self) -> Result<(), Self::Error> {
        self.start_tx.send(())?;
        Ok(())
    }

    fn shutdown(self) -> Result<(), Self::Error> {
        self.kill_tx.send(())?;
        self.handle.join()?;
        Ok(())
    }

    fn boxed_shutdown(self: Box<Self>) -> Result<(), Self::Error> {
        self.kill_tx.send(())?;
        self.handle.join()?;
        Ok(())
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

    #[allow(clippy::type_complexity)]
    pub fn build(
        self,
        genesis: &dyn BuildStorage,
    ) -> Result<ArchiveSystem<Block, ArchiveClient<Block, Executor, RA>, RA>, ArchiveError> {
        let backend = new_secondary_rocksdb_backend(self.config.client.rocksdb.clone())?;
        let backend = Arc::new(backend);
        let client =
            new_archive_client::<Block, Executor, RA>(backend.clone(), self.config.client)?;
        let client = Arc::new(client);

        Self::startup_info(&*client)?;

        let system = ArchiveSystem::new(
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
