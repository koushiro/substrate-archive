use archive_postgres::{migrate, model::*, PostgresConfig, PostgresDb, SqlxError};

#[tokio::main]
async fn main() -> Result<(), SqlxError> {
    env_logger::init();

    let config = PostgresConfig {
        uri: "postgres://koushiro:123@localhost:5432/polkadot-archive".to_string(),
        min_connections: 1,
        max_connections: 2,
        connect_timeout: 30,
        idle_timeout: Some(10 * 60),
        max_lifetime: Some(30 * 60),
        disable_statement_logging: true,
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

    let does_exist = db.check_if_metadata_exists(0).await?;
    log::info!("Metadata {} exists: {}", 0, does_exist);

    let best_block = db.best_block_num().await?;
    assert_eq!(best_block, None);
    let finalized_block = db.finalized_block_num().await?;
    assert_eq!(finalized_block, None);

    for i in 0..=u8::MAX {
        let block = BlockModel {
            spec_version: 0,
            block_num: u32::from(i),
            block_hash: vec![i],
            parent_hash: vec![i],
            state_root: vec![i],
            extrinsics_root: vec![i],
            digest: vec![i],
            extrinsics: vec![],
            justifications: Some(vec![vec![0]]),
        };
        let _ = db.insert(block).await?;

        // batch insert is much faster than insert multiple times
        let storages = (0..u8::MAX)
            .map(|count| MainStorageChangeModel {
                block_num: u32::from(i),
                block_hash: vec![i],
                prefix: vec![count],
                key: vec![count],
                data: Some(vec![count]),
            })
            .collect::<Vec<_>>();
        let _ = db.insert(storages).await?;

        let best_block = BestBlockModel {
            block_num: u32::from(i),
            block_hash: vec![i],
        };
        let _ = db.insert(best_block).await?;

        let finalized_block = FinalizedBlockModel {
            block_num: u32::from(i),
            block_hash: vec![i],
        };
        let _ = db.insert(finalized_block).await?;
    }

    let max_block_num = db.max_block_num().await?;
    log::info!("Max block num: {:?}", max_block_num);

    let best_block_num = db.best_block_num().await?.unwrap();
    let finalized_block_num = db.finalized_block_num().await?.unwrap();
    log::info!(
        "Best block num: {}, Finalized block num: {}",
        best_block_num,
        finalized_block_num,
    );

    Ok(())
}
