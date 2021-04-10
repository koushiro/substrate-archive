use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct MetadataPayload {
    pub spec_version: u32,
    pub block_num: u32,
    pub block_hash: String,
    pub meta: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct BlockPayload {
    pub spec_version: u32,
    pub block_num: u32,
    pub block_hash: String,
    pub parent_hash: String,
    pub state_root: String,
    pub extrinsics_root: String,
    pub digest: String,
    pub extrinsics: Vec<String>,

    pub storages: Vec<StorageChanges>,
}

pub type StorageChanges = (Vec<u8>, Option<Vec<u8>>);
