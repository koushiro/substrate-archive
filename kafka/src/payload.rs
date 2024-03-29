use std::collections::HashMap;

use serde::{Serialize, Serializer};

use sp_core::Bytes;
use sp_runtime::{
    generic::Digest,
    traits::{Block as BlockT, Header as HeaderT},
    Justifications,
};
use sp_storage::{StorageData, StorageKey};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataPayload<Block: BlockT> {
    pub version: u32,
    pub block_num: <Block::Header as HeaderT>::Number,
    pub block_hash: <Block::Header as HeaderT>::Hash,
    pub metadata: Bytes,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockPayload<Block: BlockT> {
    pub version: u32,
    pub block_num: <Block::Header as HeaderT>::Number,
    pub block_hash: <Block::Header as HeaderT>::Hash,
    pub parent_hash: <Block::Header as HeaderT>::Hash,
    pub state_root: <Block::Header as HeaderT>::Hash,
    pub extrinsics_root: <Block::Header as HeaderT>::Hash,
    #[serde(serialize_with = "self::serialize_digest")]
    pub digest: Digest,
    pub extrinsics: Vec<<Block as BlockT>::Extrinsic>,

    pub justifications: Option<Justifications>,

    pub main_changes: HashMap<StorageKey, Option<StorageData>>,
    pub child_changes: HashMap<StorageKey, HashMap<StorageKey, Option<StorageData>>>,
}

fn serialize_digest<D, S>(digest: &D, serializer: S) -> Result<S::Ok, S::Error>
where
    D: codec::Encode,
    S: Serializer,
{
    digest.using_encoded(|bytes| sp_core::bytes::serialize(bytes, serializer))
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BestBlockPayload<Block: BlockT> {
    pub block_num: <Block::Header as HeaderT>::Number,
    pub block_hash: <Block::Header as HeaderT>::Hash,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalizedBlockPayload<Block: BlockT> {
    pub block_num: <Block::Header as HeaderT>::Number,
    pub block_hash: <Block::Header as HeaderT>::Hash,
    pub timestamp: i64,
}

// only for example `demo`
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataPayloadForDemo {
    pub version: u32,
    pub block_num: u32,
    pub block_hash: String,
    pub metadata: Bytes,
}

// only for example `demo`
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockPayloadForDemo {
    pub version: u32,
    pub block_num: u32,
    pub block_hash: String,
    pub parent_hash: String,
    pub state_root: String,
    pub extrinsics_root: String,
    pub digest: String,
    pub extrinsics: Vec<String>,

    pub justifications: Option<Justifications>,

    pub main_changes: HashMap<StorageKey, Option<StorageData>>,
    pub child_changes: HashMap<StorageKey, HashMap<StorageKey, Option<StorageData>>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BestBlockPayloadDemo {
    pub block_num: u32,
    pub block_hash: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalizedBlockPayloadDemo {
    pub block_num: u32,
    pub block_hash: String,
    pub timestamp: i64,
}
