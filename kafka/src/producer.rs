use std::time::Duration;

use rdkafka::{
    config::ClientConfig,
    error::KafkaError,
    producer::{FutureProducer, FutureRecord},
};

use crate::{
    config::KafkaConfig,
    payload::{BlockPayload, MetadataPayload},
};

#[derive(Clone)]
pub struct KafkaProducer {
    config: KafkaConfig,
    producer: FutureProducer,
}

impl KafkaProducer {
    pub fn new(config: KafkaConfig) -> Result<Self, KafkaError> {
        assert!(
            Self::check_kafka_config(&config),
            "Invalid kafka producer configuration"
        );

        let mut client = ClientConfig::new();
        for (k, v) in &config.rdkafka {
            client.set(k, v);
        }
        let producer = client.create::<FutureProducer>()?;
        log::info!("Kafka configuration: {:?}", config);
        Ok(Self { config, producer })
    }

    fn check_kafka_config(config: &KafkaConfig) -> bool {
        (config.rdkafka.get("metadata.broker.list").is_some()
            || config.rdkafka.get("bootstrap.servers").is_some())
            && !config.topic.metadata.is_empty()
            && !config.topic.block.is_empty()
    }

    pub async fn send_metadata(&self, metadata: MetadataPayload) -> Result<(), KafkaError> {
        log::info!(
            "Kafka publish metadata, version = {}",
            metadata.spec_version
        );
        let payload =
            serde_json::to_string(&metadata).expect("serialize metadata payload shouldn't be fail");
        let key = metadata.spec_version.to_string();
        self.send(&self.config.topic.metadata, &payload, &key).await
    }

    pub async fn send_block(&self, block: BlockPayload) -> Result<(), KafkaError> {
        log::info!(
            "Kafka publish block, number = {}, hash = {}",
            block.block_num,
            block.block_hash
        );
        let payload =
            serde_json::to_string(&block).expect("serialize block payload shouldn't be fail");
        let key = block.block_num.to_string();
        self.send(&self.config.topic.block, &payload, &key).await
    }

    async fn send(&self, topic: &str, payload: &str, key: &str) -> Result<(), KafkaError> {
        let record = FutureRecord::to(topic).payload(payload).key(key);
        let queue_timeout = Duration::from_secs(self.config.queue_timeout);
        let delivery_status = self.producer.send(record, queue_timeout).await;
        match delivery_status {
            Ok(result) => {
                log::debug!(
                    "topic: {}, partition: {}, offset: {}",
                    topic,
                    result.0,
                    result.1
                );
                Ok(())
            }
            Err(err) => {
                log::error!("topic: {}, error: {}, msg: {:?}", topic, err.0, err.1);
                Err(err.0)
            }
        }
    }

    pub fn config(&self) -> &KafkaConfig {
        &self.config
    }
}
