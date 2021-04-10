use std::time::Duration;

use archive_postgres::{
    migrate, BlockModel, Insert, MetadataModel, PostgresConfig, PostgresDB, SqlxError,
};

#[tokio::main]
async fn main() -> Result<(), SqlxError> {
    env_logger::init();

    let config = PostgresConfig {
        uri: "postgres://koushiro:123@localhost:5432/archive".to_string(),
        min_connections: 1,
        max_connections: 2,
        connect_timeout: Duration::from_secs(30),
        idle_timeout: Some(Duration::from_secs(10 * 60)),
        max_lifetime: Some(Duration::from_secs(30 * 60)),
    };

    migrate(config.uri()).await?;

    let db = PostgresDB::new(config).await?;
    let mut conn = db.conn().await?;

    let metadata = MetadataModel {
        spec_version: 0,
        block_num: 0,
        block_hash: vec![0],
        meta: vec![1, 2, 3, 4, 5],
    };
    let _ = metadata.insert(&mut conn).await?;

    for i in 0..950 {
        let block = BlockModel {
            spec_version: 0,
            block_num: i,
            block_hash: vec![0],
            parent_hash: vec![0],
            state_root: vec![0],
            extrinsics_root: vec![0],
            digest: vec![0],
            extrinsics: vec![],
            storages: vec![(
                vec![(i % u32::from(u8::MAX)) as u8],
                Some(vec![(i % u32::from(u8::MAX)) as u8]),
            )],
        };
        let _ = block.insert(&mut conn).await?;
    }

    Ok(())
}
