use sqlx::{
    error::Error as SqlxError,
    pool::PoolConnection,
    postgres::{PgArguments, Postgres},
    query::Query,
};

use crate::models::{BlockModel, MetadataModel};

#[async_trait::async_trait]
pub trait Insert: Send + Sized {
    fn gen_query<'p>(self) -> Query<'p, Postgres, PgArguments>;

    async fn insert(self, conn: &mut PoolConnection<Postgres>) -> Result<u64, SqlxError>;
}

#[async_trait::async_trait]
impl Insert for MetadataModel {
    fn gen_query<'q>(self) -> Query<'q, Postgres, PgArguments> {
        sqlx::query(
            r#"
            INSERT INTO metadata VALUES ($1, $2, $3, $4)
            ON CONFLICT (spec_version)
            DO UPDATE SET (
                spec_version,
                block_num,
                block_hash,
                meta
            ) = (
                excluded.spec_version,
                excluded.block_num,
                excluded.block_hash,
                excluded.meta
            );
            "#,
        )
        .bind(self.spec_version)
        .bind(self.block_num)
        .bind(self.block_hash)
        .bind(self.meta)
    }

    async fn insert(self, conn: &mut PoolConnection<Postgres>) -> Result<u64, SqlxError> {
        log::info!(
            "Insert metadata into postgres, version = {}",
            self.spec_version
        );
        self.gen_query()
            .execute(conn)
            .await
            .map(|res| res.rows_affected())
    }
}

#[async_trait::async_trait]
impl Insert for BlockModel {
    fn gen_query<'q>(self) -> Query<'q, Postgres, PgArguments> {
        sqlx::query(
            r#"
            INSERT INTO blocks VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (block_num)
            DO UPDATE SET (
                spec_version,
                block_num,
                block_hash,
                parent_hash,
                state_root,
                extrinsics_root,
                digest,
                extrinsics,
                storages
            ) = (
                excluded.spec_version,
                excluded.block_num,
                excluded.block_hash,
                excluded.parent_hash,
                excluded.state_root,
                excluded.extrinsics_root,
                excluded.digest,
                excluded.extrinsics,
                excluded.storages
            );
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
        .bind(self.storages)
    }

    async fn insert(self, conn: &mut PoolConnection<Postgres>) -> Result<u64, SqlxError> {
        log::info!("Insert block into postgres, height = {}", self.block_num);
        self.gen_query()
            .execute(conn)
            .await
            .map(|res| res.rows_affected())
    }
}
