pub mod kafka;

use std::collections::HashMap;

use xtra::{prelude::*, spawn::TokioGlobalSpawnExt, Disconnected};

use sp_runtime::traits::{Block as BlockT, Header as HeaderT};

use crate::messages::{BlockMessage, Die, MetadataMessage};

pub trait DispatchActor<Block: BlockT>:
    Actor + Handler<MetadataMessage<Block>> + Handler<BlockMessage<Block>> + Handler<Die>
{
}

impl<Block, T> DispatchActor<Block> for T
where
    Block: BlockT,
    T: Actor + Handler<MetadataMessage<Block>> + Handler<BlockMessage<Block>> + Handler<Die>,
{
}

pub struct DispatcherActor<B: BlockT> {
    metadata_channels: HashMap<&'static str, Box<dyn StrongMessageChannel<MetadataMessage<B>>>>,
    block_channels: HashMap<&'static str, Box<dyn StrongMessageChannel<BlockMessage<B>>>>,
    die_channels: HashMap<&'static str, Box<dyn StrongMessageChannel<Die>>>,
}

impl<Block: BlockT> DispatcherActor<Block> {
    pub fn new() -> Self {
        Self {
            metadata_channels: HashMap::new(),
            block_channels: HashMap::new(),
            die_channels: HashMap::new(),
        }
    }

    pub fn add(mut self, name: &'static str, dispatcher: impl DispatchActor<Block>) -> Self {
        let addr = dispatcher.create(None).spawn_global();
        self.metadata_channels.insert(name, Box::new(addr.clone()));
        self.block_channels.insert(name, Box::new(addr.clone()));
        self.die_channels.insert(name, Box::new(addr));
        self
    }

    async fn dispatch_metadata(&self, message: MetadataMessage<Block>) -> Result<(), Disconnected> {
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

    async fn dispatch_block(&self, message: BlockMessage<Block>) -> Result<(), Disconnected> {
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
impl<Block: BlockT> Actor for DispatcherActor<Block> {}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<MetadataMessage<Block>> for DispatcherActor<Block> {
    async fn handle(
        &mut self,
        message: MetadataMessage<Block>,
        _ctx: &mut Context<Self>,
    ) -> <MetadataMessage<Block> as Message>::Result {
        if let Err(err) = self.dispatch_metadata(message).await {
            log::error!("{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<BlockMessage<Block>> for DispatcherActor<Block> {
    async fn handle(
        &mut self,
        message: BlockMessage<Block>,
        _ctx: &mut Context<Self>,
    ) -> <BlockMessage<Block> as Message>::Result {
        if let Err(err) = self.dispatch_block(message).await {
            log::error!("{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<Die> for DispatcherActor<Block> {
    async fn handle(&mut self, message: Die, ctx: &mut Context<Self>) -> <Die as Message>::Result {
        log::info!("Stopping Dispatcher Actor");
        if let Err(err) = self.dispatch_die(message).await {
            log::error!("{}", err);
        }
        ctx.stop();
    }
}
