mod actors;
mod config;
mod error;
mod exec;
mod messages;

use std::{marker::PhantomData, sync::Arc};

use sc_client_api::{
    backend::{Backend, StateBackendFor},
    client::BlockBackend,
};
use sp_api::{ApiExt, Core as CoreApi, Metadata as MetadataApi, ProvideRuntimeApi};
use sp_runtime::traits::Block as BlockT;

use self::actors::Actors;
pub use self::{
    config::{ActorConfig, DispatcherConfig, KafkaConfig, PostgresConfig},
    error::ActorError,
};

pub struct ActorSystem<Block, B> {
    backend: Arc<B>,
    start_tx: flume::Sender<()>,
    kill_tx: flume::Sender<()>,
    handle: jod_thread::JoinHandle<Result<(), ActorError>>,
    _marker: PhantomData<Block>,
}

impl<Block, B> ActorSystem<Block, B>
where
    Block: BlockT,
    B: Backend<Block> + BlockBackend<Block> + ProvideRuntimeApi<Block> + Send + Sync + 'static,
    <B as ProvideRuntimeApi<Block>>::Api: CoreApi<Block>
        + MetadataApi<Block>
        + ApiExt<Block, StateBackend = StateBackendFor<B, Block>>,
{
    pub fn new(backend: Arc<B>, config: ActorConfig) -> Self {
        let (start_tx, kill_tx, handle) = Self::start(backend.clone(), config);
        Self {
            backend,
            start_tx,
            kill_tx,
            handle,
            _marker: PhantomData,
        }
    }

    fn start(
        backend: Arc<B>,
        config: ActorConfig,
    ) -> (
        flume::Sender<()>,
        flume::Sender<()>,
        jod_thread::JoinHandle<Result<(), ActorError>>,
    ) {
        let (start_tx, start_rx) = flume::bounded(1);
        let (kill_tx, kill_rx) = flume::bounded(1);

        let runtime = tokio::runtime::Runtime::new().expect("cannot create async runtime");
        let handle = jod_thread::spawn(move || {
            let _ = start_rx.recv();
            runtime.block_on(Self::main_loop(backend, config, kill_rx))?;
            Ok(())
        });

        (start_tx, kill_tx, handle)
    }

    async fn main_loop(
        backend: Arc<B>,
        config: ActorConfig,
        kill_rx: flume::Receiver<()>,
    ) -> Result<(), ActorError> {
        let actors = Actors::spawn(backend, config).await?;
        // waiting until the kill signal is received.
        let _ = kill_rx.recv_async().await;
        actors.kill().await?;
        Ok(())
    }

    pub fn backend(&self) -> Arc<B> {
        self.backend.clone()
    }

    pub async fn drive(&mut self) -> Result<(), ActorError> {
        self.start_tx.send(())?;
        Ok(())
    }

    pub async fn shutdown(self) -> Result<(), ActorError> {
        self.kill_tx.send(())?;
        self.handle.join()?;
        Ok(())
    }

    pub async fn boxed_shutdown(self: Box<Self>) -> Result<(), ActorError> {
        self.kill_tx.send(())?;
        self.handle.join()?;
        Ok(())
    }
}
