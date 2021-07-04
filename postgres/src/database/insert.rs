use sqlx::{
    error::Error as SqlxError,
    pool::PoolConnection,
    postgres::{PgArguments, Postgres},
    query::Query,
};

use crate::{database::batch::Batch, model::*};

#[async_trait::async_trait]
pub trait InsertModel: Send + Sized {
    async fn insert(self, conn: &mut PoolConnection<Postgres>) -> Result<u64, SqlxError>;
}

#[async_trait::async_trait]
impl InsertModel for MetadataModel {
    async fn insert(self, conn: &mut PoolConnection<Postgres>) -> Result<u64, SqlxError> {
        let query: Query<'_, Postgres, PgArguments> = sqlx::query(
            r#"
            INSERT INTO metadata VALUES ($1, $2, $3, $4)
            ON CONFLICT (spec_version) DO UPDATE SET
                spec_version = EXCLUDED.spec_version,
                block_num = EXCLUDED.block_num,
                block_hash = EXCLUDED.block_hash,
                meta = EXCLUDED.meta
            "#,
        )
        .bind(self.spec_version)
        .bind(self.block_num)
        .bind(self.block_hash)
        .bind(self.meta);

        log::info!(
            target: "postgres",
            "Insert metadata into postgres, version = {}",
            self.spec_version
        );
        let rows_affected = query.execute(conn).await?.rows_affected();
        log::info!(
            target: "postgres",
            "Insert metadata into postgres, affected rows = {}",
            rows_affected
        );
        Ok(rows_affected)
    }
}

#[async_trait::async_trait]
impl InsertModel for BlockModel {
    async fn insert(self, conn: &mut PoolConnection<Postgres>) -> Result<u64, SqlxError> {
        let query: Query<'_, Postgres, PgArguments> = sqlx::query(
            r#"
            INSERT INTO block VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (block_num) DO UPDATE SET
                spec_version = EXCLUDED.spec_version,
                block_num = EXCLUDED.block_num,
                block_hash = EXCLUDED.block_hash,
                parent_hash = EXCLUDED.parent_hash,
                state_root = EXCLUDED.state_root,
                extrinsics_root = EXCLUDED.extrinsics_root,
                digest = EXCLUDED.digest,
                extrinsics = EXCLUDED.extrinsics,
                justifications = EXCLUDED.justifications
            "#,
        )
        .bind(self.spec_version)
        .bind(self.block_num)
        .bind(self.block_hash)
        .bind(self.parent_hash)
        .bind(self.state_root)
        .bind(self.extrinsics_root)
        .bind(self.digest)
        .bind(self.extrinsics)
        .bind(self.justifications);

        log::info!(
            target: "postgres",
            "Insert block into postgres, height = {}",
            self.block_num
        );
        let rows_affected = query.execute(conn).await?.rows_affected();
        log::info!(
            target: "postgres",
            "Insert block into postgres, affected rows = {}",
            rows_affected
        );
        Ok(rows_affected)
    }
}

#[async_trait::async_trait]
impl InsertModel for Vec<BlockModel> {
    async fn insert(self, conn: &mut PoolConnection<Postgres>) -> Result<u64, SqlxError> {
        log::debug!(
            target: "postgres",
            "Insert bulk block into postgres, height = [{:?}~{:?}]",
            self.first().map(|block| block.block_num),
            self.last().map(|block| block.block_num)
        );

        let mut batch = Batch::new(
            "block",
            "INSERT INTO block VALUES",
            r#"
            ON CONFLICT (block_num) DO UPDATE SET
                spec_version = EXCLUDED.spec_version,
                block_num = EXCLUDED.block_num,
                block_hash = EXCLUDED.block_hash,
                parent_hash = EXCLUDED.parent_hash,
                state_root = EXCLUDED.state_root,
                extrinsics_root = EXCLUDED.extrinsics_root,
                digest = EXCLUDED.digest,
                extrinsics = EXCLUDED.extrinsics,
                justifications = EXCLUDED.justifications
            "#,
        );
        for model in self {
            batch.reserve(9)?;
            if batch.current_num_arguments() > 0 {
                batch.append(",");
            }
            batch.append("(");
            batch.bind(model.spec_version)?;
            batch.append(",");
            batch.bind(model.block_num)?;
            batch.append(",");
            batch.bind(model.block_hash)?;
            batch.append(",");
            batch.bind(model.parent_hash)?;
            batch.append(",");
            batch.bind(model.state_root)?;
            batch.append(",");
            batch.bind(model.extrinsics_root)?;
            batch.append(",");
            batch.bind(model.digest)?;
            batch.append(",");
            batch.bind(model.extrinsics)?;
            batch.append(",");
            batch.bind(model.justifications)?;
            batch.append(")");
        }
        let rows_affected = batch.execute(conn).await?;

        log::debug!(
            target: "postgres",
            "Insert bulk block into postgres, affected rows = {}",
            rows_affected
        );
        Ok(rows_affected)
    }
}

#[async_trait::async_trait]
impl InsertModel for MainStorageChangeModel {
    async fn insert(self, conn: &mut PoolConnection<Postgres>) -> Result<u64, SqlxError> {
        log::debug!(
            target: "postgres",
            "Insert main storage into postgres, height = {}, key = 0x{}",
            self.block_num, hex::encode(&self.key)
        );

        let query: Query<'_, Postgres, PgArguments> = sqlx::query(
            r#"
            INSERT INTO main_storage VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (block_num, key) DO UPDATE SET
                block_num = EXCLUDED.block_num,
                block_hash = EXCLUDED.block_hash,
                prefix = EXCLUDED.prefix,
                key = EXCLUDED.key,
                data = EXCLUDED.data
            "#,
        )
        .bind(self.block_num)
        .bind(self.block_hash)
        .bind(self.prefix)
        .bind(self.key)
        .bind(self.data);

        let rows_affected = query.execute(conn).await?.rows_affected();
        log::debug!(
            target: "postgres",
            "Insert main storage into postgres, affected rows = {}",
            rows_affected
        );
        Ok(rows_affected)
    }
}

#[async_trait::async_trait]
impl InsertModel for Vec<MainStorageChangeModel> {
    async fn insert(self, conn: &mut PoolConnection<Postgres>) -> Result<u64, SqlxError> {
        assert!(
            !self.is_empty(),
            "main storage changes not empty for each block"
        );

        log::debug!(
            target: "postgres",
            "Insert bulk main storage into postgres, height = [{:?} ~ {:?}]",
            self.first().map(|storage| storage.block_num),
            self.last().map(|storage| storage.block_num)
        );

        /*
        let query: Query<'_, Postgres, PgArguments> = sqlx::query(
            r#"
            INSERT INTO main_storage VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (block_num, key) DO UPDATE SET
                block_num = EXCLUDED.block_num,
                block_hash = EXCLUDED.block_hash,
                prefix = EXCLUDED.prefix,
                key = EXCLUDED.key,
                data = EXCLUDED.data
            "#,
        );
        use sqlx::prelude::*;
        let mut tx = conn.begin().await?;
        let mut rows_affected = 0;
        for model in self {
            let query: Query<'_, Postgres, PgArguments> = query
                .bind(model.block_num)
                .bind(model.block_hash)
                .bind(model.prefix)
                .bind(model.key)
                .bind(model.data);
            rows_affected += query.execute(&mut tx).await?.rows_affected();
        }
        tx.commit().await?;
        */

        let mut batch = Batch::new(
            "main_storage",
            "INSERT INTO main_storage VALUES",
            r#"
            ON CONFLICT (block_num, key) DO UPDATE SET
                block_num = EXCLUDED.block_num,
                block_hash = EXCLUDED.block_hash,
                prefix = EXCLUDED.prefix,
                key = EXCLUDED.key,
                data = EXCLUDED.data
            "#,
        );
        for model in self {
            batch.reserve(5)?;
            if batch.current_num_arguments() > 0 {
                batch.append(",");
            }
            batch.append("(");
            batch.bind(model.block_num)?;
            batch.append(",");
            batch.bind(model.block_hash)?;
            batch.append(",");
            batch.bind(model.prefix)?;
            batch.append(",");
            batch.bind(model.key)?;
            batch.append(",");
            batch.bind(model.data)?;
            batch.append(")");
        }
        let rows_affected = batch.execute(conn).await?;

        log::debug!(
            target: "postgres",
            "Insert bulk main storage into postgres, affected rows = {}",
            rows_affected
        );
        Ok(rows_affected)
    }
}

#[async_trait::async_trait]
impl InsertModel for ChildStorageChangeModel {
    async fn insert(self, conn: &mut PoolConnection<Postgres>) -> Result<u64, SqlxError> {
        log::debug!(
            target: "postgres",
            "Insert child storage into postgres, height = {}, prefix key = 0x{},  key = 0x{}",
            self.block_num, hex::encode(&self.prefix_key), hex::encode(&self.key)
        );

        let query: Query<'_, Postgres, PgArguments> = sqlx::query(
            r#"
            INSERT INTO chlid_storage VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (block_num, prefix_key, key) DO UPDATE SET
                block_num = EXCLUDED.block_num,
                block_hash = EXCLUDED.block_hash,
                prefix_key = EXCLUDED.prefix_key,
                key = EXCLUDED.key,
                data = EXCLUDED.data
            "#,
        )
        .bind(self.block_num)
        .bind(self.block_hash)
        .bind(self.prefix_key)
        .bind(self.key)
        .bind(self.data);

        let rows_affected = query.execute(conn).await?.rows_affected();
        log::debug!(
            target: "postgres",
            "Insert child storage into postgres, affected rows = {}",
            rows_affected
        );
        Ok(rows_affected)
    }
}

#[async_trait::async_trait]
impl InsertModel for Vec<ChildStorageChangeModel> {
    async fn insert(self, _conn: &mut PoolConnection<Postgres>) -> Result<u64, SqlxError> {
        todo!()
    }
}

#[async_trait::async_trait]
impl InsertModel for BestBlockModel {
    async fn insert(self, conn: &mut PoolConnection<Postgres>) -> Result<u64, SqlxError> {
        log::info!(
            target: "postgres",
            "Update best block, height = {}, hash = 0x{}",
            self.block_num,
            hex::encode(&self.block_hash)
        );

        let query: Query<'_, Postgres, PgArguments> = sqlx::query(
            r#"
            INSERT INTO best_block VALUES ($1, $2, $3)
            ON CONFLICT (only_one) DO UPDATE SET
                block_num = EXCLUDED.block_num,
                block_hash = EXCLUDED.block_hash
            "#,
        )
        .bind(true)
        .bind(self.block_num)
        .bind(self.block_hash);

        let rows_affected = query.execute(conn).await?.rows_affected();
        log::info!(
            target: "postgres",
            "Update best block, affected rows = {}",
            rows_affected
        );
        Ok(rows_affected)
    }
}

#[async_trait::async_trait]
impl InsertModel for FinalizedBlockModel {
    async fn insert(self, conn: &mut PoolConnection<Postgres>) -> Result<u64, SqlxError> {
        log::info!(
            target: "postgres",
            "Update finalized block, height = {}, hash = 0x{}",
            self.block_num,
            hex::encode(&self.block_hash)
        );

        let query: Query<'_, Postgres, PgArguments> = sqlx::query(
            r#"
            INSERT INTO finalized_block VALUES ($1, $2, $3)
            ON CONFLICT (only_one) DO UPDATE SET
                block_num = EXCLUDED.block_num,
                block_hash = EXCLUDED.block_hash
            "#,
        )
        .bind(true)
        .bind(self.block_num)
        .bind(self.block_hash);

        let rows_affected = query.execute(conn).await?.rows_affected();
        log::info!(
            target: "postgres",
            "Update finalized block, affected rows = {}",
            rows_affected
        );
        Ok(rows_affected)
    }
}
