use sqlx::{
    pool::PoolConnection,
    postgres::{PgArguments, Postgres},
    Arguments, Error as SqlxError, FromRow,
};

/// Return type of queries that `SELECT EXISTS`
#[derive(Copy, Clone, Debug, Eq, PartialEq, FromRow)]
struct DoesExist {
    exists: Option<bool>,
}

pub async fn check_if_metadata_exists(
    spec_version: u32,
    conn: &mut PoolConnection<Postgres>,
) -> Result<bool, SqlxError> {
    let mut args = PgArguments::default();
    args.add(spec_version);
    let doest_exist: DoesExist = sqlx::query_as_with(
        r#"SELECT EXISTS(SELECT spec_version FROM metadatas WHERE spec_version = $1)"#,
        args,
    )
    .fetch_one(conn)
    .await?;
    Ok(doest_exist.exists.unwrap_or_default())
}

/// Return type of queries that `SELECT MAX(int)`
#[derive(Copy, Clone, Debug, Eq, PartialEq, FromRow)]
struct Max {
    max: Option<i32>,
}

pub async fn max_block_num(conn: &mut PoolConnection<Postgres>) -> Result<Option<u32>, SqlxError> {
    let max: Max = sqlx::query_as(r#"SELECT MAX(block_num) FROM blocks"#)
        .fetch_one(conn)
        .await?;
    Ok(max.max.map(|v| v as u32))
}
