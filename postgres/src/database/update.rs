use sqlx::{
    pool::PoolConnection,
    postgres::{PgArguments, Postgres},
    query::Query,
    Error as SqlxError,
};

pub async fn update_spec(
    spec_version: u32,
    block_num: u32,
    conn: &mut PoolConnection<Postgres>,
) -> Result<u64, SqlxError> {
    let query: Query<'_, Postgres, PgArguments> =
        sqlx::query(r#"UPDATE blocks SET spec_version = $1 WHERE block_num = $2"#)
            .bind(spec_version)
            .bind(block_num);

    let rows_affected = query.execute(conn).await.map(|res| res.rows_affected())?;
    Ok(rows_affected)
}
