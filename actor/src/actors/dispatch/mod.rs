pub mod kafka;

use std::collections::HashMap;

use xtra::{prelude::*, spawn::TokioGlobalSpawnExt, Disconnected};

use sp_runtime::traits::{Block as BlockT, Header as HeaderT};

use crate::types::{Block, Die, Metadata};

pub trait DispatchActor<B: BlockT>:
    Actor + Handler<Metadata<B>> + Handler<Block<B>> + Handler<Die>
{
}

impl<B, T> DispatchActor<B> for T
where
    B: BlockT,
    T: Actor + Handler<Metadata<B>> + Handler<Block<B>> + Handler<Die>,
{
}

pub struct DispatcherActor<B: BlockT> {
    metadata_channels: HashMap<&'static str, Box<dyn StrongMessageChannel<Metadata<B>>>>,
    block_channels: HashMap<&'static str, Box<dyn StrongMessageChannel<Block<B>>>>,
    die_channels: HashMap<&'static str, Box<dyn StrongMessageChannel<Die>>>,
}

impl<B: BlockT> DispatcherActor<B> {
    pub fn new() -> Self {
        Self {
            metadata_channels: HashMap::new(),
            block_channels: HashMap::new(),
            die_channels: HashMap::new(),
        }
    }

    pub fn add(mut self, name: &'static str, dispatcher: impl DispatchActor<B>) -> Self {
        let addr = dispatcher.create(None).spawn_global();
        self.metadata_channels.insert(name, Box::new(addr.clone()));
        self.block_channels.insert(name, Box::new(addr.clone()));
        self.die_channels.insert(name, Box::new(addr));
        self
    }

    async fn dispatch_metadata(&self, message: Metadata<B>) -> Result<(), Disconnected> {
        for (name, dispatcher) in &self.metadata_channels {
            log::debug!(
                "Dispatch `Metadata` message into `{}`, version = {}",
                name,
                message.spec_version
            );
            dispatcher.send(message.clone()).await?;
        }
        Ok(())
    }

    async fn dispatch_block(&self, message: Block<B>) -> Result<(), Disconnected> {
        for (name, dispatcher) in &self.block_channels {
            log::debug!(
                "Dispatch `Block` message into `{}`, height = {}",
                name,
                message.inner.block.header().number()
            );
            dispatcher.send(message.clone()).await?;
        }
        Ok(())
    }

    async fn dispatch_die(&self, message: Die) -> Result<(), Disconnected> {
        for (name, dispatcher) in &self.die_channels {
            log::debug!("Dispatch `Die` message into `{}`", name);
            dispatcher.send(message).await?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl<B: BlockT> Actor for DispatcherActor<B> {}

#[async_trait::async_trait]
impl<B: BlockT> Handler<Metadata<B>> for DispatcherActor<B> {
    async fn handle(
        &mut self,
        message: Metadata<B>,
        _ctx: &mut Context<Self>,
    ) -> <Metadata<B> as Message>::Result {
        if let Err(err) = self.dispatch_metadata(message).await {
            log::error!("{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<B: BlockT> Handler<Block<B>> for DispatcherActor<B> {
    async fn handle(
        &mut self,
        message: Block<B>,
        _ctx: &mut Context<Self>,
    ) -> <Block<B> as Message>::Result {
        if let Err(err) = self.dispatch_block(message).await {
            log::error!("{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<B: BlockT> Handler<Die> for DispatcherActor<B> {
    async fn handle(&mut self, message: Die, ctx: &mut Context<Self>) -> <Die as Message>::Result {
        log::info!("Stopping Dispatcher Actor");
        if let Err(err) = self.dispatch_die(message).await {
            log::error!("{}", err);
        }
        ctx.stop();
    }
}
