use std::time::Duration;

use archive_postgres::{migrate, BlockModel, MetadataModel, PostgresConfig, PostgresDb, SqlxError};

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

    let db = PostgresDb::new(config).await?;

    let metadata = MetadataModel {
        spec_version: 0,
        block_num: 0,
        block_hash: vec![0],
        meta: vec![1, 2, 3, 4, 5],
    };
    let _ = db.insert(metadata).await?;

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
            changes: serde_json::json!([["0x01", "0x1234"], ["0x02", null]]),
        };
        let _ = db.insert(block).await?;
    }

    Ok(())
}
