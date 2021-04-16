use anyhow::Result;

use archive::ArchiveCli;

fn main() -> Result<()> {
    let config = ArchiveCli::init()?;
    log::info!("{:#?}", config);
    Ok(())
}
