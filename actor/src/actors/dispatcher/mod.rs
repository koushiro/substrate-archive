pub mod kafka;

use std::collections::HashMap;

use futures::{future, FutureExt};
use xtra::{prelude::*, Disconnected};

use sp_runtime::traits::{Block as BlockT, Header as HeaderT};

use crate::message::{
    BatchBlockMessage, BlockMessage, Die, FinalizedBlockMessage, MetadataMessage,
};

pub trait DispatchActor<Block: BlockT>:
    Actor
    + Handler<MetadataMessage<Block>>
    + Handler<BlockMessage<Block>>
    + Handler<BatchBlockMessage<Block>>
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
        + Handler<BatchBlockMessage<Block>>
        + Handler<FinalizedBlockMessage<Block>>
        + Handler<Die>,
{
}

type MessageChannelMap<M> = HashMap<String, Box<dyn StrongMessageChannel<M>>>;

pub struct Dispatcher<Block: BlockT> {
    metadata: MessageChannelMap<MetadataMessage<Block>>,
    block: MessageChannelMap<BlockMessage<Block>>,
    blocks: MessageChannelMap<BatchBlockMessage<Block>>,
    finalized_block: MessageChannelMap<FinalizedBlockMessage<Block>>,
    die: MessageChannelMap<Die>,
}

impl<Block: BlockT> Dispatcher<Block> {
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
            block: HashMap::new(),
            blocks: HashMap::new(),
            finalized_block: HashMap::new(),
            die: HashMap::new(),
        }
    }

    pub fn add<D: DispatchActor<Block>>(
        &mut self,
        name: impl Into<String>,
        addr: Address<D>,
    ) -> &mut Self {
        let name = name.into();
        self.metadata.insert(name.clone(), Box::new(addr.clone()));
        self.block.insert(name.clone(), Box::new(addr.clone()));
        self.blocks.insert(name.clone(), Box::new(addr.clone()));
        self.finalized_block
            .insert(name.clone(), Box::new(addr.clone()));
        self.die.insert(name.clone(), Box::new(addr));
        self
    }

    pub async fn dispatch_metadata(
        &self,
        message: MetadataMessage<Block>,
    ) -> Result<(), Disconnected> {
        let results = future::join_all(self.metadata.iter().map(|(name, dispatcher)| {
            dispatcher
                .send(message.clone())
                .then(move |result| future::ready((name.clone(), result)))
        }))
        .await;
        for (name, result) in results {
            match result {
                Ok(_) => log::debug!(
                    target: "actor",
                    "Dispatch `Metadata` message into `{}`, version = {}",
                    name,
                    message.spec_version
                ),
                Err(err) => log::error!(
                    target: "actor",
                    "Failed to dispatch `Metadata` message into `{}`, version = {}: {}",
                    name,
                    message.spec_version,
                    err
                ),
            }
        }
        Ok(())
    }

    pub async fn dispatch_block(&self, message: BlockMessage<Block>) -> Result<(), Disconnected> {
        let results = future::join_all(self.block.iter().map(|(name, dispatcher)| {
            dispatcher
                .send(message.clone())
                .then(move |result| future::ready((name.clone(), result)))
        }))
        .await;
        for (name, result) in results {
            match result {
                Ok(_) => log::debug!(
                target: "actor",
                "Dispatch `Block` message into `{}`, height = {}",
                name, message.inner.block.header().number()
                ),
                Err(err) => log::error!(
                    target: "actor",
                    "Failed to dispatch `Block` message into `{}`, height = {}: {}",
                    name, message.inner.block.header().number(), err
                ),
            }
        }
        Ok(())
    }

    pub async fn dispatch_batch_block(
        &self,
        message: BatchBlockMessage<Block>,
    ) -> Result<(), Disconnected> {
        let results = future::join_all(self.blocks.iter().map(|(name, dispatcher)| {
            dispatcher
                .send(message.clone())
                .then(move |result| future::ready((name.clone(), result)))
        }))
        .await;
        for (name, result) in results {
            match result {
                Ok(_) => log::debug!(
                    target: "actor",
                    "Dispatch `BatchBlock` message into `{}`, height = [{:?}~{:?}]",
                    name,
                    message.inner().first().map(|block| block.inner.block.header().number()),
                    message.inner().last().map(|block| block.inner.block.header().number())
                ),
                Err(err) => log::error!(
                    target: "actor",
                    "Failed to dispatch `BatchBlock` message into `{}`, height = [{:?}~{:?}]: {}",
                    name,
                    message.inner().first().map(|block| block.inner.block.header().number()),
                    message.inner().last().map(|block| block.inner.block.header().number()),
                    err
                ),
            }
        }
        Ok(())
    }

    pub async fn dispatch_finalized_block(
        &self,
        message: FinalizedBlockMessage<Block>,
    ) -> Result<(), Disconnected> {
        let results = future::join_all(self.finalized_block.iter().map(|(name, dispatcher)| {
            dispatcher
                .send(message.clone())
                .then(move |result| future::ready((name.clone(), result)))
        }))
        .await;
        for (name, result) in results {
            match result {
                Ok(_) => log::debug!(
                    target: "actor",
                    "Dispatch `FinalizedBlock` message into `{}`, height = {}",
                    name, message.block_num
                ),
                Err(err) => log::error!(
                    target: "actor",
                    "Failed to dispatch `FinalizedBlock` message into `{}`, height = {}: {}",
                    name, message.block_num, err
                ),
            }
        }
        Ok(())
    }

    pub async fn dispatch_die(&self, message: Die) -> Result<(), Disconnected> {
        let results = future::join_all(self.die.iter().map(|(name, dispatcher)| {
            dispatcher
                .send(message)
                .then(move |result| future::ready((name.clone(), result)))
        }))
        .await;
        for (name, result) in results {
            match result {
                Ok(_) => log::debug!(target: "actor", "Dispatch `Die` message into `{}`", name),
                Err(err) => log::error!(
                    target: "actor",
                    "Failed to dispatch `Die` message into `{}`: {}",
                    name, err
                ),
            }
        }
        Ok(())
    }
}
