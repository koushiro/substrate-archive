use sqlx::{
    pool::PoolConnection,
    postgres::{PgArguments, Postgres},
    Arguments, Error as SqlxError, FromRow,
};

pub async fn check_if_metadata_exists(
    spec_version: u32,
    conn: &mut PoolConnection<Postgres>,
) -> Result<bool, SqlxError> {
    /// Return type of queries that `SELECT EXISTS`
    #[derive(Copy, Clone, Debug, Eq, PartialEq, FromRow)]
    struct DoesExist {
        exists: Option<bool>,
    }

    let mut args = PgArguments::default();
    args.add(spec_version);
    let doest_exist: DoesExist = sqlx::query_as_with(
        r#"SELECT EXISTS(SELECT spec_version FROM metadata WHERE spec_version = $1)"#,
        args,
    )
    .fetch_one(conn)
    .await?;
    Ok(doest_exist.exists.unwrap_or_default())
}

pub async fn get_all_metadata_versions(
    conn: &mut PoolConnection<Postgres>,
) -> Result<Vec<u32>, SqlxError> {
    #[derive(Copy, Clone, Debug, Eq, PartialEq, FromRow)]
    struct SpecVersion {
        spec_version: i32,
    }

    let versions: Vec<SpecVersion> = sqlx::query_as(r#"SELECT spec_version FROM metadata"#)
        .fetch_all(conn)
        .await?;

    Ok(versions
        .into_iter()
        .map(|result| result.spec_version as u32)
        .collect())
}

pub async fn max_block_num(conn: &mut PoolConnection<Postgres>) -> Result<Option<u32>, SqlxError> {
    /// Return type of queries that `SELECT MAX(int)`
    #[derive(Copy, Clone, Debug, Eq, PartialEq, FromRow)]
    struct Max {
        max: Option<i32>,
    }

    let max: Max = sqlx::query_as(r#"SELECT MAX(block_num) FROM block"#)
        .fetch_one(conn)
        .await?;
    Ok(max.max.map(|v| v as u32))
}

#[derive(Clone, Debug, Eq, PartialEq, FromRow)]
struct BlockForQuery {
    block_num: i32,
}

pub async fn best_block_num(conn: &mut PoolConnection<Postgres>) -> Result<Option<u32>, SqlxError> {
    let best_block: Option<BlockForQuery> = sqlx::query_as(r#"SELECT block_num FROM best_block"#)
        .fetch_optional(conn)
        .await?;
    Ok(best_block.map(|block| block.block_num as u32))
}

pub async fn finalized_block_num(
    conn: &mut PoolConnection<Postgres>,
) -> Result<Option<u32>, SqlxError> {
    let best_block: Option<BlockForQuery> =
        sqlx::query_as(r#"SELECT block_num FROM finalized_block"#)
            .fetch_optional(conn)
            .await?;
    Ok(best_block.map(|block| block.block_num as u32))
}
