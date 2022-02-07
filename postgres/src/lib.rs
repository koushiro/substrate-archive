mod config;
mod database;
pub mod model;

pub use self::{
    config::PostgresConfig,
    database::{query, PostgresDb},
    model::*,
};
pub use sqlx::error::Error as SqlxError;

/*
polkadot-archive=> select block_num, count(*) as count from main_storage group by block_num order by count desc limit 20;
 block_num | count
-----------+--------
   4876135 | 293728
   3899548 | 194735
   2005674 |  36458
         0 |  11645
   5964269 |   5678
   6798116 |   5624
   5863501 |   5555
   5878201 |   5488
   6324075 |   5467
   6769380 |   5371
   5647652 |   5312
   5834726 |   5291
   5575851 |   5281
   5892286 |   5150
   5719613 |   5137
   6280980 |   5124
   5662035 |   5111
   6668622 |   5095
   6007485 |   5083
   6553447 |   5065
(20 rows)
*/

pub async fn migrate(url: impl AsRef<str>) -> Result<(), sqlx::Error> {
    use sqlx::Connection;
    let mut conn = sqlx::PgConnection::connect(url.as_ref()).await?;
    sqlx::migrate!("./migrations").run(&mut conn).await?;
    Ok(())
}
