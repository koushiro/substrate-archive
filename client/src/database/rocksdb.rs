use std::{collections::HashMap, fmt, io, path::PathBuf};

use kvdb::{DBValue, IoStatsKind, KeyValueDB};
use kvdb_rocksdb::{Database, DatabaseConfig};

use sp_database::{ColumnId, Database as DatabaseT, Transaction};

use crate::{columns, database::DbHash, utils::NUM_COLUMNS};

pub struct SecondaryRocksDB(Database);
impl fmt::Debug for SecondaryRocksDB {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let stats = self.0.io_stats(IoStatsKind::Overall);
        write!(f, "Read Only Database Stats: {:?}", stats)
    }
}

impl SecondaryRocksDB {
    pub fn open(path: PathBuf, cache_size: usize, secondary_db_path: PathBuf) -> io::Result<Self> {
        let path = path.to_str().expect("cannot find primary rocksdb");
        let secondary_db_path = secondary_db_path
            .to_str()
            .expect("cannot create secondary rocksdb db");

        let mut db_config = DatabaseConfig::with_columns(NUM_COLUMNS);
        db_config.secondary = Some(secondary_db_path.to_string());

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
            "Open RocksDB database at {}, state column budget: {} MiB, others({}) column cache: {} MiB",
            path,
            state_col_budget,
            NUM_COLUMNS,
            other_col_budget,
        );
        // light node database
        /*
        let col_budget = cache_size / (NUM_COLUMNS as usize);
        for i in 0..NUM_COLUMNS {
            memory_budget.insert(i, col_budget);
        }
        log::trace!(
            "Open RocksDB light database at {}, column cache: {} MiB",
            path,
            col_budget,
        );
        */

        db_config.memory_budget = memory_budget;

        let db = Database::open(&db_config, path)?;
        db.try_catch_up_with_primary()?;
        Ok(Self(db))
    }

    pub fn get(&self, col: ColumnId, key: &[u8]) -> Option<DBValue> {
        match self.0.get(col, key) {
            Ok(value) => value,
            Err(err) => {
                log::debug!("{}, Catching up with primary and trying again...", err);
                self.0.try_catch_up_with_primary().ok()?;
                match self.0.get(col, key) {
                    Ok(value) => value,
                    Err(err) => {
                        log::error!("{}", err);
                        None
                    }
                }
            }
        }
    }
}

type DatabaseResult<T> = sp_database::error::Result<T>;

impl DatabaseT<DbHash> for SecondaryRocksDB {
    fn commit(&self, _transaction: Transaction<DbHash>) -> DatabaseResult<()> {
        panic!("Read Only Database")
    }

    fn get(&self, col: u32, key: &[u8]) -> Option<Vec<u8>> {
        self.get(col, key)
    }
}
