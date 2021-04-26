use xtra::prelude::*;

use sp_runtime::traits::Block as BlockT;

use archive_kafka::{BlockPayload, KafkaConfig, KafkaError, KafkaProducer, MetadataPayload};

use crate::messages::{BlockMessage, Die, MetadataMessage};

pub struct KafkaActor<Block: BlockT> {
    producer: KafkaProducer,
    _marker: std::marker::PhantomData<Block>,
}

impl<B: BlockT> KafkaActor<B> {
    pub fn new(config: KafkaConfig) -> Result<Self, KafkaError> {
        let producer = KafkaProducer::new(config)?;
        Ok(Self {
            producer,
            _marker: std::marker::PhantomData,
        })
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Actor for KafkaActor<Block> {}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<MetadataMessage<Block>> for KafkaActor<Block> {
    async fn handle(
        &mut self,
        message: MetadataMessage<Block>,
        _: &mut Context<Self>,
    ) -> <MetadataMessage<Block> as Message>::Result {
        let payload = MetadataPayload::from(message);
        if let Err(err) = self.producer.send(payload).await {
            log::error!(target: "actor", "{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<BlockMessage<Block>> for KafkaActor<Block> {
    async fn handle(
        &mut self,
        message: BlockMessage<Block>,
        _: &mut Context<Self>,
    ) -> <BlockMessage<Block> as Message>::Result {
        let payload = BlockPayload::from(message);
        if let Err(err) = self.producer.send(payload).await {
            log::error!(target: "actor", "{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<Block: BlockT> Handler<Die> for KafkaActor<Block> {
    async fn handle(&mut self, _message: Die, ctx: &mut Context<Self>) -> <Die as Message>::Result {
        log::info!(target: "actor", "Stopping Kafka Actor");
        ctx.stop();
    }
}
