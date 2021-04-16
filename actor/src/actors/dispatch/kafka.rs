use xtra::prelude::*;

use sp_runtime::traits::Block as BlockT;

use archive_kafka::{BlockPayload, KafkaConfig, KafkaError, KafkaProducer, MetadataPayload};

use crate::types::{Block, Die, Metadata};

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
impl<B: BlockT> Handler<Metadata<B>> for KafkaActor<B> {
    async fn handle(
        &mut self,
        message: Metadata<B>,
        _ctx: &mut Context<Self>,
    ) -> <Metadata<B> as Message>::Result {
        let payload = MetadataPayload::from(message);
        if let Err(err) = self.producer.send(payload).await {
            log::error!("{}", err);
        }
    }
}

#[async_trait::async_trait]
impl<B: BlockT> Handler<Block<B>> for KafkaActor<B> {
    async fn handle(
        &mut self,
        message: Block<B>,
        _ctx: &mut Context<Self>,
    ) -> <Block<B> as Message>::Result {
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
