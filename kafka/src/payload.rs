use serde::Serialize;

use sp_runtime::{
    generic::Digest,
    traits::{Block as BlockT, Header as HeaderT},
    Justifications,
};
use sp_storage::{StorageData, StorageKey};

#[derive(Clone, Debug, Serialize)]
pub struct MetadataPayload<B: BlockT> {
    pub spec_version: u32,
    pub block_num: <B::Header as HeaderT>::Number,
    pub block_hash: <B::Header as HeaderT>::Hash,
    #[serde(with = "serde_bytes")]
    pub meta: Vec<u8>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BlockPayload<B: BlockT> {
    pub spec_version: u32,
    pub block_num: <B::Header as HeaderT>::Number,
    pub block_hash: <B::Header as HeaderT>::Hash,
    pub parent_hash: <B::Header as HeaderT>::Hash,
    pub state_root: <B::Header as HeaderT>::Hash,
    pub extrinsics_root: <B::Header as HeaderT>::Hash,
    pub digest: Digest<<B::Header as HeaderT>::Hash>,
    pub extrinsics: Vec<<B as BlockT>::Extrinsic>,

    pub justifications: Option<Justifications>,

    pub changes: Vec<(StorageKey, Option<StorageData>)>,
}

// only for example `demo`
#[derive(Clone, Debug, Serialize)]
pub struct MetadataPayloadForDemo {
    pub spec_version: u32,
    pub block_num: u32,
    pub block_hash: String,
    pub meta: String,
}

// only for example `demo`
#[derive(Clone, Debug, Serialize)]
pub struct BlockPayloadForDemo {
    pub spec_version: u32,
    pub block_num: u32,
    pub block_hash: String,
    pub parent_hash: String,
    pub state_root: String,
    pub extrinsics_root: String,
    pub digest: String,
    pub extrinsics: Vec<String>,

    pub justifications: Option<Justifications>,

    pub changes: Vec<(StorageKey, Option<StorageData>)>,
}
