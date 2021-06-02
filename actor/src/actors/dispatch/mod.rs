pub mod kafka;

use std::collections::HashMap;

use xtra::{prelude::*, Disconnected};

use sp_runtime::traits::{Block as BlockT, Header as HeaderT};

use crate::message::{BlockMessage, Die, MetadataMessage};

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

pub struct Dispatcher<Block: BlockT> {
    metadata_channels: HashMap<&'static str, Box<dyn StrongMessageChannel<MetadataMessage<Block>>>>,
    block_channels: HashMap<&'static str, Box<dyn StrongMessageChannel<BlockMessage<Block>>>>,
    die_channels: HashMap<&'static str, Box<dyn StrongMessageChannel<Die>>>,
}

impl<Block: BlockT> Dispatcher<Block> {
    pub fn new() -> Self {
        Self {
            metadata_channels: HashMap::new(),
            block_channels: HashMap::new(),
            die_channels: HashMap::new(),
        }
    }

    pub fn add<D: DispatchActor<Block>>(
        &mut self,
        name: &'static str,
        addr: Address<D>,
    ) -> &mut Self {
        self.metadata_channels.insert(name, Box::new(addr.clone()));
        self.block_channels.insert(name, Box::new(addr.clone()));
        self.die_channels.insert(name, Box::new(addr));
        self
    }

    pub async fn dispatch_metadata(
        &self,
        message: MetadataMessage<Block>,
    ) -> Result<(), Disconnected> {
        for (name, dispatcher) in &self.metadata_channels {
            log::debug!(
                target: "actor",
                "Dispatch `Metadata` message into `{}`, version = {}",
                name,
                message.spec_version
            );
            dispatcher.send(message.clone()).await?;
        }
        Ok(())
    }

    pub async fn dispatch_block(&self, message: BlockMessage<Block>) -> Result<(), Disconnected> {
        for (name, dispatcher) in &self.block_channels {
            log::debug!(
                target: "actor",
                "Dispatch `Block` message into `{}`, height = {}",
                name,
                message.inner.block.header().number()
            );
            dispatcher.send(message.clone()).await?;
        }
        Ok(())
    }

    pub async fn dispatch_die(&self, message: Die) -> Result<(), Disconnected> {
        for (name, dispatcher) in &self.die_channels {
            log::debug!(target: "actor", "Dispatch `Die` message into `{}`", name);
            dispatcher.send(message).await?;
        }
        Ok(())
    }
}
