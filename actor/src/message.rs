use std::{collections::HashMap, marker::PhantomData};

use codec::Encode;
use sp_blockchain::well_known_cache_keys;
use sp_runtime::{
    generic::SignedBlock,
    traits::{Block as BlockT, Header as HeaderT},
    SaturatedConversion,
};
use sp_state_machine::{ChildStorageCollection, StorageCollection};
use sp_storage::{well_known_keys, StorageData, StorageKey};

use crate::error::{ActorError, SqlxError};

// ============================================================================
// `Data` Actor Message
// ============================================================================

#[derive(Clone, Debug)]
pub struct MetadataMessage<Block: BlockT> {
    pub version: u32,
    pub block_num: <Block::Header as HeaderT>::Number,
    pub block_hash: <Block::Header as HeaderT>::Hash,
    pub metadata: Vec<u8>,
}

impl<Block: BlockT> xtra::Message for MetadataMessage<Block> {
    type Result = ();
}

impl<Block: BlockT> From<MetadataMessage<Block>> for archive_postgres::MetadataModel {
    fn from(metadata: MetadataMessage<Block>) -> Self {
        Self {
            version: metadata.version,
            block_num: metadata.block_num.saturated_into(),
            block_hash: metadata.block_hash.as_ref().to_vec(),
            metadata: metadata.metadata,
        }
    }
}

impl<Block: BlockT> From<MetadataMessage<Block>> for archive_kafka::MetadataPayload<Block> {
    fn from(metadata: MetadataMessage<Block>) -> Self {
        Self {
            version: metadata.version,
            block_num: metadata.block_num,
            block_hash: metadata.block_hash,
            metadata: metadata.metadata.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BlockMessage<Block: BlockT> {
    pub version: u32,
    pub inner: SignedBlock<Block>,
    pub main_changes: StorageCollection,
    pub child_changes: ChildStorageCollection,
}

impl<Block: BlockT> xtra::Message for BlockMessage<Block> {
    type Result = ();
}

fn is_well_known_key(key: &[u8]) -> bool {
    const CACHE_KEY_AUTH: &[u8] = &well_known_cache_keys::AUTHORITIES;
    const CACHE_KEY_EPOCH: &[u8] = &well_known_cache_keys::EPOCH;
    const CACHE_KEY_CHANGES_TRIE_CONFIG: &[u8] = &well_known_cache_keys::CHANGES_TRIE_CONFIG;
    matches!(
        key,
        well_known_keys::CODE
            | well_known_keys::HEAP_PAGES
            | well_known_keys::EXTRINSIC_INDEX
            | well_known_keys::CHILD_STORAGE_KEY_PREFIX
            | well_known_keys::DEFAULT_CHILD_STORAGE_KEY_PREFIX
            | CACHE_KEY_AUTH
            | CACHE_KEY_EPOCH
            | CACHE_KEY_CHANGES_TRIE_CONFIG
            | sp_finality_grandpa::GRANDPA_AUTHORITIES_KEY
    )
}

fn into_block_model<Block: BlockT>(
    version: u32,
    block_num: u32,
    block_hash: Vec<u8>,
    block: SignedBlock<Block>,
) -> archive_postgres::BlockModel {
    archive_postgres::BlockModel {
        version,
        block_num,
        block_hash: block_hash.clone(),
        parent_hash: block.block.header().parent_hash().as_ref().to_vec(),
        state_root: block.block.header().state_root().as_ref().to_vec(),
        extrinsics_root: block.block.header().extrinsics_root().as_ref().to_vec(),
        digest: block.block.header().digest().encode(),
        extrinsics: block
            .block
            .extrinsics()
            .iter()
            .map(|ext| ext.encode())
            .collect(),
        justifications: block.justifications.map(|justifications| {
            justifications
                .into_iter()
                .map(|justification| justification.encode())
                .collect()
        }),
    }
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
            into_block_model(block.version, block_num, block_hash.clone(), block.inner),
            into_main_storage_models(block_num, block_hash.clone(), block.main_changes),
            into_child_storage_models(block_num, block_hash, block.child_changes),
        )
    }
}

impl<Block: BlockT> From<BlockMessage<Block>> for archive_kafka::BlockPayload<Block> {
    fn from(block: BlockMessage<Block>) -> Self {
        Self {
            version: block.version,
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
pub struct BatchBlockMessage<Block: BlockT> {
    pub inner: Vec<BlockMessage<Block>>,
}

impl<Block: BlockT> BatchBlockMessage<Block> {
    pub fn new(blocks: Vec<BlockMessage<Block>>) -> Self {
        Self { inner: blocks }
    }

    pub fn inner(&self) -> &[BlockMessage<Block>] {
        &self.inner
    }

    pub fn into_inner(self) -> Vec<BlockMessage<Block>> {
        self.inner
    }
}

impl<Block: BlockT> xtra::Message for BatchBlockMessage<Block> {
    type Result = ();
}

impl<Block: BlockT> From<BatchBlockMessage<Block>>
    for (
        Vec<archive_postgres::BlockModel>,
        Vec<archive_postgres::MainStorageChangeModel>,
        Vec<archive_postgres::ChildStorageChangeModel>,
    )
{
    fn from(message: BatchBlockMessage<Block>) -> Self {
        let mut blocks = Vec::with_capacity(message.inner().len());
        let mut main_storages = Vec::with_capacity(message.inner().len());
        let mut child_storages = Vec::with_capacity(message.inner().len());
        for block in message.into_inner() {
            let block_num = (*block.inner.block.header().number()).saturated_into();
            let block_hash = block.inner.block.header().hash().as_ref().to_vec();
            blocks.push(into_block_model(
                block.version,
                block_num,
                block_hash.clone(),
                block.inner,
            ));
            main_storages.extend(into_main_storage_models(
                block_num,
                block_hash.clone(),
                block.main_changes,
            ));
            child_storages.extend(into_child_storage_models(
                block_num,
                block_hash,
                block.child_changes,
            ));
        }
        (blocks, main_storages, child_storages)
    }
}

#[derive(Clone, Debug)]
pub struct BestBlockMessage<Block: BlockT> {
    pub block_num: <Block::Header as HeaderT>::Number,
    pub block_hash: Block::Hash,
}

impl<Block: BlockT> xtra::Message for BestBlockMessage<Block> {
    type Result = ();
}

impl<Block: BlockT> From<BestBlockMessage<Block>> for archive_postgres::BestBlockModel {
    fn from(best_block: BestBlockMessage<Block>) -> Self {
        Self {
            block_num: best_block.block_num.saturated_into(),
            block_hash: best_block.block_hash.as_ref().to_vec(),
        }
    }
}

impl<Block: BlockT> From<BestBlockMessage<Block>> for archive_kafka::BestBlockPayload<Block> {
    fn from(best_block: BestBlockMessage<Block>) -> Self {
        Self {
            block_num: best_block.block_num,
            block_hash: best_block.block_hash,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FinalizedBlockMessage<Block: BlockT> {
    pub block_num: <Block::Header as HeaderT>::Number,
    pub block_hash: Block::Hash,
    pub timestamp: i64,
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
            timestamp: finalized_block.timestamp,
        }
    }
}

// ============================================================================
// `Communication` Actor Message
// ============================================================================

#[derive(Copy, Clone, Debug)]
pub struct DbIfMetadataExist {
    pub version: u32,
}
impl xtra::Message for DbIfMetadataExist {
    type Result = Result<bool, SqlxError>;
}

#[derive(Copy, Clone, Debug)]
pub struct DbMaxBlock;
impl xtra::Message for DbMaxBlock {
    type Result = Result<Option<u32>, SqlxError>;
}

#[derive(Copy, Clone, Debug)]
pub struct DbBestBlock;
impl xtra::Message for DbBestBlock {
    type Result = Result<Option<(u32, Vec<u8>)>, SqlxError>;
}

#[derive(Copy, Clone, Debug)]
pub struct DbFinalizedBlock;
impl xtra::Message for DbFinalizedBlock {
    type Result = Result<Option<(u32, Vec<u8>)>, SqlxError>;
}

#[derive(Copy, Clone, Debug)]
pub struct DbDeleteGtBlockNum {
    pub block_num: u32,
}
impl DbDeleteGtBlockNum {
    pub fn new(block_num: u32) -> Self {
        Self { block_num }
    }
}
impl xtra::Message for DbDeleteGtBlockNum {
    type Result = Result<u64, SqlxError>;
}

#[derive(Copy, Clone, Debug)]
pub struct CatchupFinalized;
impl xtra::Message for CatchupFinalized {
    type Result = ();
}

#[derive(Copy, Clone, Debug)]
pub struct BestAndFinalized;
impl xtra::Message for BestAndFinalized {
    // best_block_num, finalized_block_num
    type Result = (u32, u32);
}

// ============================================================================
// `Command` Actor Message
// ============================================================================

#[derive(Copy, Clone, Debug)]
pub struct Initialize;
impl xtra::Message for Initialize {
    type Result = ();
}

#[derive(Copy, Clone, Debug)]
pub struct Tick;
impl xtra::Message for Tick {
    type Result = Result<(), ActorError>;
}

#[derive(Copy, Clone, Debug)]
pub struct CrawlBestAndFinalized;
impl xtra::Message for CrawlBestAndFinalized {
    type Result = ();
}

#[derive(Copy, Clone, Debug)]
pub struct CrawlBlock<Block: BlockT>(u32, PhantomData<Block>);
impl<Block: BlockT> CrawlBlock<Block> {
    pub fn new(block: u32) -> Self {
        Self(block, PhantomData)
    }

    pub fn block_num(&self) -> u32 {
        self.0
    }
}
impl<Block: BlockT> xtra::Message for CrawlBlock<Block> {
    type Result = Option<BlockMessage<Block>>;
}

#[derive(Copy, Clone, Debug)]
pub struct Die;
impl xtra::Message for Die {
    type Result = ();
}
