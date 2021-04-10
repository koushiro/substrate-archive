use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct MetadataModel {
    pub spec_version: u32,
    pub block_num: u32,
    pub block_hash: Vec<u8>,
    pub meta: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct BlockModel {
    pub spec_version: u32,
    pub block_num: u32,
    pub block_hash: Vec<u8>,
    pub parent_hash: Vec<u8>,
    pub state_root: Vec<u8>,
    pub extrinsics_root: Vec<u8>,
    pub digest: Vec<u8>,
    pub extrinsics: Vec<Vec<u8>>,

    pub storages: Vec<StorageChanges>,
}

pub type StorageChanges = (Vec<u8>, Option<Vec<u8>>);

/*
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct StorageChanges {
    // pub spec_version: u32,
    // pub block_num: u32,
    // pub block_hash: Vec<u8>,
    pub key: Vec<u8>,
    pub data: Option<Vec<u8>>,
}
*/
