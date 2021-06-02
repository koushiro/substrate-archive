use std::collections::HashMap;

use codec::Encode;
use sp_runtime::{
    generic::SignedBlock,
    traits::{Block as BlockT, Header as HeaderT},
    SaturatedConversion,
};
use sp_state_machine::{ChildStorageCollection, StorageCollection};
use sp_storage::{well_known_keys, StorageData, StorageKey};

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

fn is_well_known_key(key: &[u8]) -> bool {
    matches!(
        key,
        well_known_keys::CODE
            | well_known_keys::HEAP_PAGES
            | well_known_keys::EXTRINSIC_INDEX
            | well_known_keys::CHANGES_TRIE_CONFIG
            | well_known_keys::CHILD_STORAGE_KEY_PREFIX
            | well_known_keys::DEFAULT_CHILD_STORAGE_KEY_PREFIX
            | sp_finality_grandpa::GRANDPA_AUTHORITIES_KEY
    )
}

fn into_main_storage_models(
    block_num: u32,
    block_hash: Vec<u8>,
    main_changes: StorageCollection,
) -> Vec<archive_postgres::MainStorageChangeModel> {
    main_changes
        .into_iter()
        .map(|(key, data)| {
            if is_well_known_key(&key) {
                archive_postgres::MainStorageChangeModel {
                    block_num,
                    block_hash: block_hash.clone(),
                    prefix: key.clone(),
                    key,
                    data,
                }
            } else {
                archive_postgres::MainStorageChangeModel {
                    block_num,
                    block_hash: block_hash.clone(),
                    // Twox128(module_prefix) ++ Twox128(storage_prefix) = 32 bytes
                    // e.g: prefix = 0b76934f4cc08dee01012d059e1b83ee5e0621c4869aa60c02be9adcc98a0d1d
                    prefix: key.as_slice()[..32].to_vec(),
                    key,
                    data,
                }
            }
        })
        .collect()
}

fn into_child_storage_models(
    block_num: u32,
    block_hash: Vec<u8>,
    child_changes: ChildStorageCollection,
) -> Vec<archive_postgres::ChildStorageChangeModel> {
    child_changes
        .into_iter()
        .map(|(prefix_key, kv)| {
            kv.into_iter()
                .map(|(key, data)| archive_postgres::ChildStorageChangeModel {
                    block_num,
                    block_hash: block_hash.clone(),
                    prefix_key: prefix_key.clone(),
                    key,
                    data,
                })
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect()
}

impl<Block: BlockT> From<BlockMessage<Block>>
    for (
        archive_postgres::BlockModel,
        Vec<archive_postgres::MainStorageChangeModel>,
        Vec<archive_postgres::ChildStorageChangeModel>,
    )
{
    fn from(block: BlockMessage<Block>) -> Self {
        let block_num = (*block.inner.block.header().number()).saturated_into();
        let block_hash = block.inner.block.header().hash().as_ref().to_vec();
        (
            archive_postgres::BlockModel {
                spec_version: block.spec_version,
                block_num,
                block_hash: block_hash.clone(),
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
            },
            into_main_storage_models(block_num, block_hash.clone(), block.main_changes),
            into_child_storage_models(block_num, block_hash, block.child_changes),
        )
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
            main_changes: block
                .main_changes
                .into_iter()
                .map(|(k, v)| (StorageKey(k), v.map(StorageData)))
                .collect(),
            child_changes: block
                .child_changes
                .into_iter()
                .map(|(k, v)| {
                    (
                        StorageKey(k),
                        v.into_iter()
                            .map(|(k, v)| (StorageKey(k), v.map(StorageData)))
                            .collect::<HashMap<_, _>>(),
                    )
                })
                .collect::<HashMap<_, _>>(),
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
