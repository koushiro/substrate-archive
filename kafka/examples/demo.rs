use std::collections::HashMap;

use archive_kafka::{
    BlockPayload, KafkaConfig, KafkaError, KafkaProducer, KafkaTopicConfig, MetadataPayload,
};

#[tokio::main]
async fn main() -> Result<(), KafkaError> {
    env_logger::init();

    let config = KafkaConfig {
        queue_timeout: 0,
        topic: KafkaTopicConfig {
            metadata: "polkadot-metadata".into(),
            block: "polkadot-block".into(),
        },
        rdkafka: {
            let mut rdkakfa = HashMap::new();
            rdkakfa.insert("bootstrap.servers".into(), "localhost:9092".into());
            rdkakfa.insert("compression.codec".into(), "none".into());
            rdkakfa
        },
    };

    let producer = KafkaProducer::new(config)?;

    let metadata = MetadataPayload {
        spec_version: 0,
        block_num: 0,
        block_hash: "0x00".into(),
        meta: "0x0102030405".into(),
    };
    producer.send_metadata(metadata).await?;

    for i in 0..950 {
        let block = BlockPayload {
            spec_version: 0,
            block_num: i,
            block_hash: "0x00".into(),
            parent_hash: "0x00".into(),
            state_root: "0x00".into(),
            extrinsics_root: "0x00".into(),
            digest: "0x00".into(),
            extrinsics: vec![],
            storages: vec![(i.to_string(), Some(i.to_string()))],
        };
        producer.send_block(block).await?
    }

    Ok(())
}
