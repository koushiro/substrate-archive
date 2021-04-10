use std::sync::Arc;

use codec::{Decode, Encode};
use sp_blockchain::{
    Backend as BlockchainBackend, BlockStatus, Cache, CachedHeaderMetadata, HeaderBackend,
    HeaderMetadata, HeaderMetadataCache, Info,
};
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, Header as HeaderT, NumberFor},
    Justifications,
};

use crate::{
    columns,
    database::{DbHash, ReadOnlyDb},
    error::{backend_err, BlockchainError, BlockchainResult},
    utils::{self, meta_keys},
};

/// Block body storage scheme.
#[derive(Debug, Clone, Copy)]
pub enum TransactionStorageMode {
    /// Store block body as an encoded list of full transactions in the BODY column
    BlockBody,
    /// Store a list of hashes in the BODY column and each transaction individually
    /// in the TRANSACTION column.
    StorageChain,
}

/// This is used as block body when storage-chain mode is enabled.
#[derive(Debug, Encode, Decode)]
struct ExtrinsicHeader {
    /// Hash of the indexed part
    indexed_hash: DbHash, // Zero hash if there's no indexed data
    /// The rest of the data.
    data: Vec<u8>,
}

// Block database
pub struct BlockchainDb<Block: BlockT> {
    db: Arc<dyn ReadOnlyDb>,
    // meta: Arc<RwLock<Meta<NumberFor<Block>, Block::Hash>>>,
    header_metadata_cache: Arc<HeaderMetadataCache<Block>>,
    transaction_storage: TransactionStorageMode,
}

impl<Block> BlockchainDb<Block>
where
    Block: BlockT,
{
    pub fn new(
        db: Arc<dyn ReadOnlyDb>,
        transaction_storage: TransactionStorageMode,
    ) -> BlockchainResult<Self> {
        Ok(Self {
            db,
            header_metadata_cache: Arc::new(HeaderMetadataCache::default()),
            transaction_storage,
        })
    }
}

impl<Block> BlockchainBackend<Block> for BlockchainDb<Block>
where
    Block: BlockT,
{
    fn body(&self, id: BlockId<Block>) -> BlockchainResult<Option<Vec<Block::Extrinsic>>> {
        let body = match utils::read_db(&*self.db, columns::KEY_LOOKUP, columns::BODY, id)? {
            Some(body) => body,
            None => return Ok(None),
        };

        match self.transaction_storage {
            TransactionStorageMode::BlockBody => match Decode::decode(&mut body.as_ref()) {
                Ok(body) => Ok(Some(body)),
                Err(err) => Err(backend_err(format!("Error decoding body: {}", err))),
            },
            TransactionStorageMode::StorageChain => {
                match Vec::<ExtrinsicHeader>::decode(&mut body.as_ref()) {
                    Ok(index) => {
                        let extrinsics: BlockchainResult<Vec<Block::Extrinsic>> = index
                            .into_iter()
                            .map(|ExtrinsicHeader { indexed_hash, data }| {
                                let decode_result = if indexed_hash != Default::default() {
                                    match self.db.get(columns::TRANSACTION, indexed_hash.as_ref()) {
                                        Some(t) => Block::Extrinsic::decode(
                                            &mut utils::join_input(data.as_ref(), t.as_ref()),
                                        ),
                                        None => {
                                            return Err(backend_err(format!(
                                                "Missing indexed transaction {:?}",
                                                indexed_hash
                                            )))
                                        }
                                    }
                                } else {
                                    Block::Extrinsic::decode(&mut data.as_ref())
                                };
                                decode_result.map_err(|err| {
                                    backend_err(format!("Error decoding extrinsic: {}", err))
                                })
                            })
                            .collect();
                        Ok(Some(extrinsics?))
                    }
                    Err(err) => Err(backend_err(format!("Error decoding body list: {}", err))),
                }
            }
        }
    }

    fn justifications(&self, id: BlockId<Block>) -> BlockchainResult<Option<Justifications>> {
        match utils::read_db(&*self.db, columns::KEY_LOOKUP, columns::JUSTIFICATIONS, id)? {
            Some(justification) => match Decode::decode(&mut justification.as_ref()) {
                Ok(justifications) => Ok(Some(justifications)),
                Err(err) => Err(backend_err(format!(
                    "Error decoding justification: {}",
                    err
                ))),
            },
            None => Ok(None),
        }
    }

    fn last_finalized(&self) -> BlockchainResult<Block::Hash> {
        Ok(utils::read_meta::<Block>(&*self.db, columns::HEADER)?.finalized_hash)
    }

    fn cache(&self) -> Option<Arc<dyn Cache<Block>>> {
        None
    }

    fn leaves(&self) -> BlockchainResult<Vec<Block::Hash>> {
        unimplemented!()
        // LeafSet::read_from_db(&*self.db, columns::META, meta_keys::LEAF_PREFIX)
        //     .map(|leaves| leaves.hashes())
    }

    fn children(&self, parent_hash: Block::Hash) -> BlockchainResult<Vec<Block::Hash>> {
        utils::read_children(
            &*self.db,
            columns::META,
            meta_keys::CHILDREN_PREFIX,
            parent_hash,
        )
    }

    fn indexed_transaction(&self, hash: &Block::Hash) -> BlockchainResult<Option<Vec<u8>>> {
        Ok(self.db.get(columns::TRANSACTION, hash.as_ref()))
    }
}

impl<Block> HeaderBackend<Block> for BlockchainDb<Block>
where
    Block: BlockT,
{
    fn header(&self, id: BlockId<Block>) -> BlockchainResult<Option<Block::Header>> {
        utils::read_header(&*self.db, columns::KEY_LOOKUP, columns::HEADER, id)
    }

    fn info(&self) -> Info<Block> {
        // TODO: Remove expect
        let meta = utils::read_meta::<Block>(&*self.db, columns::HEADER)
            .expect("Metadata could not be read");
        log::warn!("Leaves are not counted on the read-only backend!");
        Info {
            best_hash: meta.best_hash,
            best_number: meta.best_number,
            genesis_hash: meta.genesis_hash,
            finalized_hash: meta.finalized_hash,
            finalized_number: meta.finalized_number,
            number_leaves: 0,
        }
    }

    fn status(&self, id: BlockId<Block>) -> BlockchainResult<BlockStatus> {
        // log::warn!("Read-only backend does not track Block Status!");
        // Ok(BlockStatus::Unknown)
        let exists = match id {
            BlockId::Hash(_) => self.header(id)?.is_some(),
            BlockId::Number(n) => {
                n <= utils::read_meta::<Block>(&*self.db, columns::HEADER)?.best_number
            }
        };
        Ok(if exists {
            BlockStatus::InChain
        } else {
            BlockStatus::Unknown
        })
    }

    fn number(
        &self,
        hash: Block::Hash,
    ) -> BlockchainResult<Option<<<Block as BlockT>::Header as HeaderT>::Number>> {
        Ok(self
            .header_metadata(hash)
            .ok()
            .map(|header_metadata| header_metadata.number))
    }

    fn hash(&self, number: NumberFor<Block>) -> BlockchainResult<Option<Block::Hash>> {
        Ok(self.header(BlockId::Number(number))?.map(|h| h.hash()))
    }
}

impl<Block> HeaderMetadata<Block> for BlockchainDb<Block>
where
    Block: BlockT,
{
    type Error = BlockchainError;

    fn header_metadata(
        &self,
        hash: Block::Hash,
    ) -> Result<CachedHeaderMetadata<Block>, Self::Error> {
        match self.header_metadata_cache.header_metadata(hash) {
            Some(header_metadata) => Ok(header_metadata),
            None => self
                .header(BlockId::Hash(hash))?
                .map(|header| {
                    let header_metadata = CachedHeaderMetadata::from(&header);
                    self.header_metadata_cache
                        .insert_header_metadata(hash, header_metadata.clone());
                    header_metadata
                })
                .ok_or_else(|| {
                    BlockchainError::UnknownBlock(format!("header not found in db: {}", hash))
                }),
        }
    }

    fn insert_header_metadata(
        &self,
        hash: Block::Hash,
        header_metadata: CachedHeaderMetadata<Block>,
    ) {
        self.header_metadata_cache
            .insert_header_metadata(hash, header_metadata)
    }

    fn remove_header_metadata(&self, hash: Block::Hash) {
        self.header_metadata_cache.remove_header_metadata(hash);
    }
}
