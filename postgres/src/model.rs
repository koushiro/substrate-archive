use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct MetadataModel {
    pub version: u32,
    pub block_num: u32,
    pub block_hash: Vec<u8>,
    pub metadata: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct BlockModel {
    pub version: u32,
    pub block_num: u32,
    pub block_hash: Vec<u8>,
    pub parent_hash: Vec<u8>,
    pub state_root: Vec<u8>,
    pub extrinsics_root: Vec<u8>,
    pub digest: Vec<u8>,
    pub extrinsics: Vec<Vec<u8>>,

    pub justifications: Option<Vec<Vec<u8>>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct MainStorageChangeModel {
    pub block_num: u32,
    pub block_hash: Vec<u8>,
    pub prefix: Vec<u8>,
    pub key: Vec<u8>,
    pub data: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct ChildStorageChangeModel {
    pub block_num: u32,
    pub block_hash: Vec<u8>,
    pub prefix_key: Vec<u8>,
    pub key: Vec<u8>,
    pub data: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct BestBlockModel {
    pub block_num: u32,
    pub block_hash: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct FinalizedBlockModel {
    pub block_num: u32,
    pub block_hash: Vec<u8>,
}
