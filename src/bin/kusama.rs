use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use archive::{ArchiveCli, ArchiveError, ArchiveSystemBuilder, Block};

sc_executor::native_executor_instance!(
    pub KusamaExecutor,
    kusama_runtime::api::dispatch,
    kusama_runtime::native_version,
    frame_benchmarking::benchmarking::HostFunctions,
);

type KusamaArchiveSystemBuilder =
    ArchiveSystemBuilder<Block, KusamaExecutor, kusama_runtime::RuntimeApi>;

fn main() -> Result<(), ArchiveError> {
    let config = ArchiveCli::init()?;
    log::info!(target: "archive", "{:#?}", config);

    let archive = KusamaArchiveSystemBuilder::with_config(config).build()?;
    archive.drive()?;

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {}
    archive.shutdown()?;

    Ok(())
}
