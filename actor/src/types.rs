use codec::Encode;
use sp_runtime::{
    generic::SignedBlock,
    traits::{Block as BlockT, Header as HeaderT},
    SaturatedConversion,
};
use sp_storage::{StorageData, StorageKey};

#[derive(Clone, Debug)]
pub struct Metadata<B: BlockT> {
    pub spec_version: u32,
    pub block_num: <B::Header as HeaderT>::Number,
    pub block_hash: <B::Header as HeaderT>::Hash,
    pub meta: Vec<u8>,
}

impl<B: BlockT> xtra::Message for Metadata<B> {
    type Result = ();
}

impl<B: BlockT> From<Metadata<B>> for archive_postgres::MetadataModel {
    fn from(metadata: Metadata<B>) -> Self {
        Self {
            spec_version: metadata.spec_version,
            block_num: metadata.block_num.saturated_into(),
            block_hash: metadata.block_hash.as_ref().to_vec(),
            meta: metadata.meta,
        }
    }
}

impl<B: BlockT> From<Metadata<B>> for archive_kafka::MetadataPayload<B> {
    fn from(metadata: Metadata<B>) -> Self {
        Self {
            spec_version: metadata.spec_version,
            block_num: metadata.block_num,
            block_hash: metadata.block_hash,
            meta: metadata.meta,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Block<B: BlockT> {
    pub spec_version: u32,
    pub inner: SignedBlock<B>,
    pub changes: Vec<(StorageKey, Option<StorageData>)>,
}

impl<B: BlockT> xtra::Message for Block<B> {
    type Result = ();
}

impl<B: BlockT> From<Block<B>> for archive_postgres::BlockModel {
    fn from(block: Block<B>) -> Self {
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
            changes: serde_json::to_value(block.changes)
                .expect("Serialize storage changes shouldn't be fail"),
        }
    }
}

impl<B: BlockT> From<Block<B>> for archive_kafka::BlockPayload<B> {
    fn from(block: Block<B>) -> Self {
        Self {
            spec_version: block.spec_version,
            block_num: *block.inner.block.header().number(),
            block_hash: block.inner.block.header().hash(),
            parent_hash: *block.inner.block.header().parent_hash(),
            state_root: *block.inner.block.header().state_root(),
            extrinsics_root: *block.inner.block.header().extrinsics_root(),
            digest: block.inner.block.header().digest().clone(),
            extrinsics: block.inner.block.extrinsics().to_vec(),
            changes: block.changes,
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
pub struct Die;

impl xtra::Message for Die {
    type Result = ();
}
