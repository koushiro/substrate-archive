mod rocksdb;

pub use self::rocksdb::SecondaryRocksDb;

/// Hash type that backend uses for the database.
pub type DbHash = sp_core::H256;

pub trait ReadOnlyDb: sp_database::Database<DbHash> {}
impl<T> ReadOnlyDb for T where T: sp_database::Database<DbHash> {}
