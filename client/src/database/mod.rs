mod rocksdb;

pub use self::rocksdb::SecondaryRocksDB;

/// Hash type that backend uses for the database.
pub type DbHash = sp_core::H256;

pub trait ReadOnlyDB: sp_database::Database<DbHash> {}
impl<T> ReadOnlyDB for T where T: sp_database::Database<DbHash> {}
