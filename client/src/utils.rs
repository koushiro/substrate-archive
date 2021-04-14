// Copy from sc-client-db utils and children modules, since those functions are not public

use std::{convert::TryInto, hash::Hash};

use codec::{Decode, Encode};
use sc_client_db::DbHash;
use sp_database::Database;
use sp_runtime::{
    generic::BlockId,
    traits::{
        Block as BlockT, Header as HeaderT, NumberFor, UniqueSaturatedFrom, UniqueSaturatedInto,
        Zero,
    },
};

use crate::error::{backend_err, BlockchainError};

/// Number of columns in the db. Must be the same for both full && light dbs.
/// Otherwise RocksDb will fail to open database && check its type.
pub const NUM_COLUMNS: u32 = 12;
/// Meta column. The set of keys in the column is shared by full && light storages.
pub const COLUMN_META: u32 = 0;

/// Keys of entries in COLUMN_META.
#[allow(unused)]
pub mod meta_keys {
    /// Type of storage (full or light).
    pub const TYPE: &[u8; 4] = b"type";
    /// Best block key.
    pub const BEST_BLOCK: &[u8; 4] = b"best";
    /// Last finalized block key.
    pub const FINALIZED_BLOCK: &[u8; 5] = b"final";
    /// Meta information prefix for list-based caches.
    pub const CACHE_META_PREFIX: &[u8; 5] = b"cache";
    /// Meta information for changes tries key.
    pub const CHANGES_TRIES_META: &[u8; 5] = b"ctrie";
    /// Genesis block hash.
    pub const GENESIS_HASH: &[u8; 3] = b"gen";
    /// Leaves prefix list key.
    pub const LEAF_PREFIX: &[u8; 4] = b"leaf";
    /// Children prefix list key.
    pub const CHILDREN_PREFIX: &[u8; 8] = b"children";
}

/// Database metadata.
#[derive(Debug)]
pub struct Meta<N, H> {
    /// Hash of the best known block.
    pub best_hash: H,
    /// Number of the best known block.
    pub best_number: N,
    /// Hash of the best finalized block.
    pub finalized_hash: H,
    /// Number of the best finalized block.
    pub finalized_number: N,
    /// Hash of the genesis block.
    pub genesis_hash: H,
}

/// A block lookup key: used for canonical lookup from block number to hash
pub type NumberIndexKey = [u8; 4];

/// Convert block number into short lookup key (LE representation) for
/// blocks that are in the canonical chain.
///
/// In the current database schema, this kind of key is only used for
/// lookups into an index, NOT for storing header data or others.
pub fn number_index_key<N: TryInto<u32>>(n: N) -> Result<NumberIndexKey, BlockchainError> {
    let n = n
        .try_into()
        .map_err(|_| backend_err("Block number cannot be converted to u32"))?;

    Ok([
        (n >> 24) as u8,
        ((n >> 16) & 0xff) as u8,
        ((n >> 8) & 0xff) as u8,
        (n & 0xff) as u8,
    ])
}

/// Convert block id to block lookup key.
/// block lookup key is the DB-key header, block and justification are stored under.
/// looks up lookup key by hash from DB as necessary.
pub fn block_id_to_lookup_key<Block>(
    db: &dyn Database<DbHash>,
    key_lookup_col: u32,
    id: BlockId<Block>,
) -> Result<Option<Vec<u8>>, BlockchainError>
where
    Block: BlockT,
    NumberFor<Block>: UniqueSaturatedFrom<u64> + UniqueSaturatedInto<u64>,
{
    Ok(match id {
        BlockId::Number(n) => db.get(key_lookup_col, number_index_key(n)?.as_ref()),
        BlockId::Hash(h) => db.get(key_lookup_col, h.as_ref()),
    })
}

/// Read database column entry for the given block.
pub fn read_db<Block>(
    db: &dyn Database<DbHash>,
    col_index: u32,
    col: u32,
    id: BlockId<Block>,
) -> Result<Option<Vec<u8>>, BlockchainError>
where
    Block: BlockT,
{
    block_id_to_lookup_key(db, col_index, id).map(|key| match key {
        Some(key) => db.get(col, key.as_ref()),
        None => None,
    })
}

/// Read a header from the database.
pub fn read_header<Block: BlockT>(
    db: &dyn Database<DbHash>,
    col_index: u32,
    col: u32,
    id: BlockId<Block>,
) -> Result<Option<Block::Header>, BlockchainError> {
    match read_db(db, col_index, col, id)? {
        Some(header) => match Block::Header::decode(&mut &header[..]) {
            Ok(header) => Ok(Some(header)),
            Err(_) => Err(backend_err("Error decoding header")),
        },
        None => Ok(None),
    }
}

/// Read genesis hash from database.
pub fn read_genesis_hash<Hash: Decode>(
    db: &dyn Database<DbHash>,
) -> Result<Option<Hash>, BlockchainError> {
    match db.get(COLUMN_META, meta_keys::GENESIS_HASH) {
        Some(h) => match Decode::decode(&mut &h[..]) {
            Ok(h) => Ok(Some(h)),
            Err(err) => Err(backend_err(format!("Error decoding genesis hash: {}", err))),
        },
        None => Ok(None),
    }
}

/// Read meta from the database.
pub fn read_meta<Block>(
    db: &dyn Database<DbHash>,
    col_header: u32,
) -> Result<Meta<<<Block as BlockT>::Header as HeaderT>::Number, Block::Hash>, BlockchainError>
where
    Block: BlockT,
{
    let genesis_hash: Block::Hash = match read_genesis_hash(db)? {
        Some(genesis_hash) => genesis_hash,
        None => {
            return Ok(Meta {
                best_hash: Default::default(),
                best_number: Zero::zero(),
                finalized_hash: Default::default(),
                finalized_number: Zero::zero(),
                genesis_hash: Default::default(),
            })
        }
    };

    let load_meta_block = |desc, key| -> Result<_, BlockchainError> {
        if let Some(Some(header)) = match db.get(COLUMN_META, key) {
            Some(id) => db
                .get(col_header, &id)
                .map(|b| Block::Header::decode(&mut &b[..]).ok()),
            None => None,
        } {
            let hash = header.hash();
            log::debug!(
                "Opened blockchain db, fetched {} = {:?} ({})",
                desc,
                hash,
                header.number()
            );
            Ok((hash, *header.number()))
        } else {
            Ok((genesis_hash, Zero::zero()))
        }
    };

    let (best_hash, best_number) = load_meta_block("best", meta_keys::BEST_BLOCK)?;
    let (finalized_hash, finalized_number) = load_meta_block("final", meta_keys::FINALIZED_BLOCK)?;

    Ok(Meta {
        best_hash,
        best_number,
        finalized_hash,
        finalized_number,
        genesis_hash,
    })
}

/// Returns the hashes of the children blocks of the block with `parent_hash`.
pub fn read_children<K, V>(
    db: &dyn Database<DbHash>,
    column: u32,
    prefix: &[u8],
    parent_hash: K,
) -> Result<Vec<V>, BlockchainError>
where
    K: Eq + Hash + Clone + Encode + Decode,
    V: Eq + Hash + Clone + Encode + Decode,
{
    let mut buf = prefix.to_vec();
    parent_hash.using_encoded(|s| buf.extend(s));

    let raw_val_opt = db.get(column, buf.as_ref());
    let raw_val = match raw_val_opt {
        Some(val) => val,
        None => return Ok(Vec::new()),
    };

    let children: Vec<V> = match Decode::decode(&mut raw_val.as_ref()) {
        Ok(children) => children,
        Err(_) => return Err(backend_err("Error decoding children")),
    };

    Ok(children)
}

pub(crate) struct JoinInput<'a, 'b>(&'a [u8], &'b [u8]);

pub(crate) fn join_input<'a, 'b>(i1: &'a [u8], i2: &'b [u8]) -> JoinInput<'a, 'b> {
    JoinInput(i1, i2)
}

impl<'a, 'b> codec::Input for JoinInput<'a, 'b> {
    fn remaining_len(&mut self) -> Result<Option<usize>, codec::Error> {
        Ok(Some(self.0.len() + self.1.len()))
    }

    fn read(&mut self, into: &mut [u8]) -> Result<(), codec::Error> {
        let mut read = 0;
        if !self.0.is_empty() {
            read = std::cmp::min(self.0.len(), into.len());
            self.0.read(&mut into[..read])?;
        }
        if read < into.len() {
            self.1.read(&mut into[read..])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_input_works() {
        use codec::Input;

        let buf1 = [1, 2, 3, 4];
        let buf2 = [5, 6, 7, 8];
        let mut test = [0, 0, 0];
        let mut joined = join_input(buf1.as_ref(), buf2.as_ref());
        assert_eq!(joined.remaining_len().unwrap(), Some(8));

        joined.read(&mut test).unwrap();
        assert_eq!(test, [1, 2, 3]);
        assert_eq!(joined.remaining_len().unwrap(), Some(5));

        joined.read(&mut test).unwrap();
        assert_eq!(test, [4, 5, 6]);
        assert_eq!(joined.remaining_len().unwrap(), Some(2));

        joined.read(&mut test[0..2]).unwrap();
        assert_eq!(test, [7, 8, 6]);
        assert_eq!(joined.remaining_len().unwrap(), Some(0));
    }
}
