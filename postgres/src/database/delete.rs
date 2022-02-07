use sqlx::{error::Error as SqlxError, pool::PoolConnection, postgres::Postgres};

use crate::model::*;

#[async_trait::async_trait]
pub trait DeleteModel: Send + Sized {
    async fn delete(conn: &mut PoolConnection<Postgres>, block_num: u32) -> Result<u64, SqlxError>;
}

fn gen_delete_sql(table: &str) -> String {
    format!("DELETE FROM {} WHERE block_num > $1", table)
}

#[async_trait::async_trait]
impl DeleteModel for MetadataModel {
    async fn delete(conn: &mut PoolConnection<Postgres>, block_num: u32) -> Result<u64, SqlxError> {
        let sql = gen_delete_sql("metadata");
        let query = sqlx::query(&sql).bind(block_num);
        let rows_affected = query.execute(conn).await?.rows_affected();
        log::info!(
            target: "postgres",
            "Delete metadata (block_num > {}) from postgres, affected rows = {}",
            block_num,
            rows_affected
        );
        Ok(rows_affected)
    }
}

#[async_trait::async_trait]
impl DeleteModel for BlockModel {
    async fn delete(conn: &mut PoolConnection<Postgres>, block_num: u32) -> Result<u64, SqlxError> {
        let sql = gen_delete_sql("block");
        let query = sqlx::query(&sql).bind(block_num);
        let rows_affected = query.execute(conn).await?.rows_affected();
        log::info!(
            target: "postgres",
            "Delete block (block_num > {}) from postgres, affected rows = {}",
            block_num,
            rows_affected
        );
        Ok(rows_affected)
    }
}

#[async_trait::async_trait]
impl DeleteModel for MainStorageChangeModel {
    async fn delete(conn: &mut PoolConnection<Postgres>, block_num: u32) -> Result<u64, SqlxError> {
        let sql = gen_delete_sql("main_storage");
        let query = sqlx::query(&sql).bind(block_num);
        let rows_affected = query.execute(conn).await?.rows_affected();
        log::info!(
            target: "postgres",
            "Delete main_storage (block_num > {}) from postgres, affected rows = {}",
            block_num,
            rows_affected
        );
        Ok(rows_affected)
    }
}
