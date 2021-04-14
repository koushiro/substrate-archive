use std::collections::HashMap;

use archive_kafka::{
    BlockPayloadForDemo, KafkaConfig, KafkaError, KafkaProducer, KafkaTopicConfig, MetadataPayload,
    StorageData, StorageKey,
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
    producer.send(metadata).await?;

    for i in 0..950 {
        let block = BlockPayloadForDemo {
            spec_version: 0,
            block_num: i,
            block_hash: "0x00".into(),
            parent_hash: "0x00".into(),
            state_root: "0x00".into(),
            extrinsics_root: "0x00".into(),
            digest: "0x00".into(),
            extrinsics: vec![],
            changes: vec![(
                StorageKey(vec![(i % u32::from(u8::MAX)) as u8]),
                Some(StorageData(vec![(i % u32::from(u8::MAX)) as u8])),
            )],
        };
        producer.send(block).await?
    }

    Ok(())
}
