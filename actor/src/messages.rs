use std::collections::HashMap;

use codec::Encode;
use sp_runtime::{
    generic::SignedBlock,
    traits::{Block as BlockT, Header as HeaderT},
    SaturatedConversion,
};
use sp_storage::{StorageData, StorageKey};

#[derive(Clone, Debug)]
pub struct MetadataMessage<Block: BlockT> {
    pub spec_version: u32,
    pub block_num: <Block::Header as HeaderT>::Number,
    pub block_hash: <Block::Header as HeaderT>::Hash,
    pub meta: Vec<u8>,
}

impl<Block: BlockT> xtra::Message for MetadataMessage<Block> {
    type Result = ();
}

impl<Block: BlockT> From<MetadataMessage<Block>> for archive_postgres::MetadataModel {
    fn from(metadata: MetadataMessage<Block>) -> Self {
        Self {
            spec_version: metadata.spec_version,
            block_num: metadata.block_num.saturated_into(),
            block_hash: metadata.block_hash.as_ref().to_vec(),
            meta: metadata.meta,
        }
    }
}

impl<Block: BlockT> From<MetadataMessage<Block>> for archive_kafka::MetadataPayload<Block> {
    fn from(metadata: MetadataMessage<Block>) -> Self {
        Self {
            spec_version: metadata.spec_version,
            block_num: metadata.block_num,
            block_hash: metadata.block_hash,
            meta: metadata.meta.into(),
        }
    }
}

/// A list of top trie storage data.
pub type StorageCollection = HashMap<StorageKey, Option<StorageData>>;
/// A list of children trie storage data.
/// The key does not including prefix, for the `default`
/// trie kind, so this is exclusively for the `ChildType::ParentKeyId`
/// tries.
pub type ChildStorageCollection = HashMap<StorageKey, StorageCollection>;

#[derive(Clone, Debug)]
pub struct BlockMessage<Block: BlockT> {
    pub spec_version: u32,
    pub inner: SignedBlock<Block>,
    pub main_changes: StorageCollection,
    pub child_changes: ChildStorageCollection,
}

impl<Block: BlockT> xtra::Message for BlockMessage<Block> {
    type Result = ();
}

impl<Block: BlockT> From<BlockMessage<Block>> for archive_postgres::BlockModel {
    fn from(block: BlockMessage<Block>) -> Self {
        Self {
            spec_version: block.spec_version,
            block_num: (*block.inner.block.header().number()).saturated_into(),
            block_hash: block.inner.block.header().hash().as_ref().to_vec(),
            parent_hash: block.inner.block.header().parent_hash().as_ref().to_vec(),
            state_root: block.inner.block.header().state_root().as_ref().to_vec(),
            extrinsics_root: block
                .inner
                .block
                .header()
                .extrinsics_root()
                .as_ref()
                .to_vec(),
            digest: block.inner.block.header().digest().encode(),
            extrinsics: block
                .inner
                .block
                .extrinsics()
                .iter()
                .map(|ext| ext.encode())
                .collect(),
            justifications: block.inner.justifications.map(|justifications| {
                justifications
                    .into_iter()
                    .map(|justification| justification.encode())
                    .collect()
            }),
            main_changes: serde_json::to_value(block.main_changes)
                .expect("Serialize storage changes shouldn't be fail"),
            child_changes: serde_json::to_value(block.child_changes)
                .expect("Serialize child storage changes shouldn't be fail"),
        }
    }
}

impl<Block: BlockT> From<BlockMessage<Block>> for archive_kafka::BlockPayload<Block> {
    fn from(block: BlockMessage<Block>) -> Self {
        Self {
            spec_version: block.spec_version,
            block_num: *block.inner.block.header().number(),
            block_hash: block.inner.block.header().hash(),
            parent_hash: *block.inner.block.header().parent_hash(),
            state_root: *block.inner.block.header().state_root(),
            extrinsics_root: *block.inner.block.header().extrinsics_root(),
            digest: block.inner.block.header().digest().clone(),
            extrinsics: block.inner.block.extrinsics().to_vec(),
            justifications: block.inner.justifications,
            main_changes: block.main_changes,
            child_changes: block.child_changes,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FinalizedBlockMessage<Block: BlockT> {
    pub block_num: <Block::Header as HeaderT>::Number,
    pub block_hash: Block::Hash,
}

impl<Block: BlockT> xtra::Message for FinalizedBlockMessage<Block> {
    type Result = ();
}

impl<Block: BlockT> From<FinalizedBlockMessage<Block>> for archive_postgres::FinalizedBlockModel {
    fn from(finalized_block: FinalizedBlockMessage<Block>) -> Self {
        Self {
            block_num: finalized_block.block_num.saturated_into(),
            block_hash: finalized_block.block_hash.as_ref().to_vec(),
        }
    }
}

impl<Block: BlockT> From<FinalizedBlockMessage<Block>>
    for archive_kafka::FinalizedBlockPayload<Block>
{
    fn from(finalized_block: FinalizedBlockMessage<Block>) -> Self {
        Self {
            block_num: finalized_block.block_num,
            block_hash: finalized_block.block_hash,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CheckIfMetadataExist {
    pub spec_version: u32,
}

impl xtra::Message for CheckIfMetadataExist {
    type Result = bool;
}

#[derive(Copy, Clone, Debug)]
pub struct MaxBlock;

impl xtra::Message for MaxBlock {
    type Result = Option<u32>;
}

#[derive(Copy, Clone, Debug)]
pub struct ReIndex;

impl xtra::Message for ReIndex {
    type Result = ();
}

#[derive(Copy, Clone, Debug)]
pub struct Crawl;

impl xtra::Message for Crawl {
    type Result = ();
}

#[derive(Copy, Clone, Debug)]
pub struct Die;

impl xtra::Message for Die {
    type Result = ();
}
