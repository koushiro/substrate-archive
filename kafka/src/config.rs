use std::collections::HashMap;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct KafkaConfig {
    pub queue_timeout: u64, // seconds
    pub topic: KafkaTopicConfig,
    pub rdkafka: HashMap<String, String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct KafkaTopicConfig {
    pub metadata: String,
    pub block: String,
}
