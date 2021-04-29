use serde::Serialize;

use sp_core::Bytes;
use sp_runtime::{
    generic::Digest,
    traits::{Block as BlockT, Header as HeaderT},
    Justifications,
};
use sp_storage::{StorageData, StorageKey};

#[derive(Clone, Debug, Serialize)]
pub struct MetadataPayload<Block: BlockT> {
    pub spec_version: u32,
    pub block_num: <Block::Header as HeaderT>::Number,
    pub block_hash: <Block::Header as HeaderT>::Hash,
    pub meta: Bytes,
}

/// A list of top trie storage data.
pub type StorageCollection = Vec<(StorageKey, Option<StorageData>)>;
/// A list of children trie storage data.
/// The key does not including prefix, for the `default`
/// trie kind, so this is exclusively for the `ChildType::ParentKeyId`
/// tries.
pub type ChildStorageCollection = Vec<(StorageKey, StorageCollection)>;

#[derive(Clone, Debug, Serialize)]
pub struct BlockPayload<Block: BlockT> {
    pub spec_version: u32,
    pub block_num: <Block::Header as HeaderT>::Number,
    pub block_hash: <Block::Header as HeaderT>::Hash,
    pub parent_hash: <Block::Header as HeaderT>::Hash,
    pub state_root: <Block::Header as HeaderT>::Hash,
    pub extrinsics_root: <Block::Header as HeaderT>::Hash,
    pub digest: Digest<<Block::Header as HeaderT>::Hash>,
    pub extrinsics: Vec<<Block as BlockT>::Extrinsic>,

    pub justifications: Option<Justifications>,

    pub changes: StorageCollection,
    pub child_changes: ChildStorageCollection,
}

#[derive(Clone, Debug, Serialize)]
pub struct FinalizedBlockPayload<Block: BlockT> {
    pub block_num: <Block::Header as HeaderT>::Number,
    pub block_hash: <Block::Header as HeaderT>::Hash,
}

// only for example `demo`
#[derive(Clone, Debug, Serialize)]
pub struct MetadataPayloadForDemo {
    pub spec_version: u32,
    pub block_num: u32,
    pub block_hash: String,
    pub meta: Bytes,
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

    pub changes: StorageCollection,
    pub child_changes: ChildStorageCollection,
}
