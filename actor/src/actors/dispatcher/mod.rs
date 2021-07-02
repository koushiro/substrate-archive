pub mod kafka;

use std::collections::HashMap;

use xtra::{prelude::*, Disconnected};

use sp_runtime::traits::{Block as BlockT, Header as HeaderT};

use crate::message::{BestBlockMessage, BlockMessage, Die, FinalizedBlockMessage, MetadataMessage};

pub trait DispatchActor<Block: BlockT>:
    Actor
    + Handler<MetadataMessage<Block>>
    + Handler<BlockMessage<Block>>
    + Handler<BestBlockMessage<Block>>
    + Handler<FinalizedBlockMessage<Block>>
    + Handler<Die>
{
}

impl<Block, T> DispatchActor<Block> for T
where
    Block: BlockT,
    T: Actor
        + Handler<MetadataMessage<Block>>
        + Handler<BlockMessage<Block>>
        + Handler<BestBlockMessage<Block>>
        + Handler<FinalizedBlockMessage<Block>>
        + Handler<Die>,
{
}

type MessageChannelMap<M> = HashMap<&'static str, Box<dyn StrongMessageChannel<M>>>;

pub struct Dispatcher<Block: BlockT> {
    metadata: MessageChannelMap<MetadataMessage<Block>>,
    block: MessageChannelMap<BlockMessage<Block>>,
    best_block: MessageChannelMap<BestBlockMessage<Block>>,
    finalized_block: MessageChannelMap<FinalizedBlockMessage<Block>>,
    die: MessageChannelMap<Die>,
}

impl<Block: BlockT> Dispatcher<Block> {
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
            block: HashMap::new(),
            best_block: HashMap::new(),
            finalized_block: HashMap::new(),
            die: HashMap::new(),
        }
    }

    pub fn add<D: DispatchActor<Block>>(
        &mut self,
        name: &'static str,
        addr: Address<D>,
    ) -> &mut Self {
        self.metadata.insert(name, Box::new(addr.clone()));
        self.block.insert(name, Box::new(addr.clone()));
        self.best_block.insert(name, Box::new(addr.clone()));
        self.finalized_block.insert(name, Box::new(addr.clone()));
        self.die.insert(name, Box::new(addr));
        self
    }

    pub async fn dispatch_metadata(
        &self,
        message: MetadataMessage<Block>,
    ) -> Result<(), Disconnected> {
        for (name, dispatcher) in &self.metadata {
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
        for (name, dispatcher) in &self.block {
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

    pub async fn dispatch_best_block(
        &self,
        message: BestBlockMessage<Block>,
    ) -> Result<(), Disconnected> {
        for (name, dispatcher) in &self.best_block {
            log::debug!(
                target: "actor",
                "Dispatch `BestBlock` message into `{}`, height = {}",
                name,
                message.block_num
            );
            dispatcher.send(message.clone()).await?;
        }
        Ok(())
    }

    pub async fn dispatch_finalized_block(
        &self,
        message: FinalizedBlockMessage<Block>,
    ) -> Result<(), Disconnected> {
        for (name, dispatcher) in &self.finalized_block {
            log::debug!(
                target: "actor",
                "Dispatch `FinalizedBlock` message into `{}`, height = {}",
                name,
                message.block_num
            );
            dispatcher.send(message.clone()).await?;
        }
        Ok(())
    }

    pub async fn dispatch_die(&self, message: Die) -> Result<(), Disconnected> {
        for (name, dispatcher) in &self.die {
            log::debug!(target: "actor", "Dispatch `Die` message into `{}`", name);
            dispatcher.send(message).await?;
        }
        Ok(())
    }
}
