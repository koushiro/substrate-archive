use xtra::prelude::*;

use sp_runtime::traits::Block as BlockT;

use archive_kafka::{BlockPayload, KafkaConfig, KafkaError, KafkaProducer, MetadataPayload};

use crate::messages::{BlockMessage, Die, MetadataMessage};

pub struct KafkaActor<B: BlockT> {
    producer: KafkaProducer,
    _marker: std::marker::PhantomData<B>,
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
impl<B: BlockT> Actor for KafkaActor<B> {}

#[async_trait::async_trait]
impl<B: BlockT> Handler<MetadataMessage<B>> for KafkaActor<B> {
    async fn handle(
        &mut self,
        message: MetadataMessage<B>,
        _: &mut Context<Self>,
    ) -> <MetadataMessage<B> as Message>::Result {
        let payload = MetadataPayload::from(message);
        if let Err(err) = self.producer.send(payload).await {
            log::error!("{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<B: BlockT> Handler<BlockMessage<B>> for KafkaActor<B> {
    async fn handle(
        &mut self,
        message: BlockMessage<B>,
        _: &mut Context<Self>,
    ) -> <BlockMessage<B> as Message>::Result {
        let payload = BlockPayload::from(message);
        if let Err(err) = self.producer.send(payload).await {
            log::error!("{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<B: BlockT> Handler<Die> for KafkaActor<B> {
    async fn handle(&mut self, _message: Die, ctx: &mut Context<Self>) -> <Die as Message>::Result {
        log::info!("Stopping Kafka Actor");
        ctx.stop();
    }
}
