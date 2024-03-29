use std::{collections::HashMap, fmt, io, path::PathBuf};

use kvdb::{DBValue, IoStatsKind, KeyValueDB};
use kvdb_rocksdb::{Database, DatabaseConfig};
use serde::{Deserialize, Serialize};

use sc_client_db::DbHash;
use sp_database::{ColumnId, Database as DatabaseT, Transaction};

use crate::{columns, utils::NUM_COLUMNS};

type DatabaseResult<T> = sp_database::error::Result<T>;

/// Secondary rocksdb configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RocksDbConfig {
    pub path: PathBuf,
    pub cache_size: usize,
    pub secondary_db_path: PathBuf,
}

pub struct SecondaryRocksDb(Database);
impl fmt::Debug for SecondaryRocksDb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let stats = self.0.io_stats(IoStatsKind::Overall);
        write!(f, "Read Only Database Stats: {:?}", stats)
    }
}

impl SecondaryRocksDb {
    pub fn open(config: RocksDbConfig) -> io::Result<Self> {
        let path = config.path.to_str().expect("Cannot find primary rocksdb");
        let cache_size = config.cache_size;

        let mut db_config = DatabaseConfig::with_columns(NUM_COLUMNS);
        db_config.secondary = Some(config.secondary_db_path);
        // `max_open_files` is useless for rocksdb secondary instance.
        // db_config.max_open_files = config.max_open_files;

        let mut memory_budget = HashMap::new();
        // Full node database.
        let state_col_budget = (cache_size as f64 * 0.9) as usize;
        let other_col_budget = (cache_size - state_col_budget) / (NUM_COLUMNS as usize - 1);
        for i in 0..NUM_COLUMNS {
            if i == columns::STATE {
                memory_budget.insert(i, state_col_budget);
            } else {
                memory_budget.insert(i, other_col_budget);
            }
        }
        log::info!(
            target: "client",
            "Open RocksDB database at {}, state column budget: {} MiB, others({}) column cache: {} MiB",
            path,
            state_col_budget,
            NUM_COLUMNS,
            other_col_budget,
        );

        db_config.memory_budget = memory_budget;

        let db = Database::open(&db_config, path)?;
        db.try_catch_up_with_primary()?;
        Ok(Self(db))
    }

    pub fn get(&self, col: ColumnId, key: &[u8]) -> Option<DBValue> {
        match self.0.get(col, key) {
            Ok(Some(value)) => Some(value),
            Ok(None) => {
                self.0.try_catch_up_with_primary().ok()?;
                None
            }
            Err(err) => {
                log::debug!(target: "client", "{}, Catching up with primary and trying again...", err);
                self.0.try_catch_up_with_primary().ok()?;
                match self.0.get(col, key) {
                    Ok(value) => value,
                    Err(err) => {
                        log::error!(target: "client", "{}", err);
                        None
                    }
                }
            }
        }
    }
}

impl DatabaseT<DbHash> for SecondaryRocksDb {
    fn commit(&self, _transaction: Transaction<DbHash>) -> DatabaseResult<()> {
        panic!("Read-only database don't support commit transaction")
    }

    fn get(&self, col: u32, key: &[u8]) -> Option<Vec<u8>> {
        self.get(col, key)
    }
}
