mod config;
mod database;
mod models;

pub use self::{
    config::PostgresConfig,
    database::{insert::Insert, query, PostgresDB},
    models::{BlockModel, MetadataModel, StorageChanges},
};
pub use sqlx::error::Error as SqlxError;

pub async fn migrate(url: impl AsRef<str>) -> Result<(), sqlx::Error> {
    use sqlx::Connection;
    let mut conn = sqlx::PgConnection::connect(url.as_ref()).await?;
    sqlx::migrate!("./migrations").run(&mut conn).await?;
    Ok(())
}
