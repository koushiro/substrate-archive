use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use sc_executor::native_executor_instance;
use sp_runtime::{generic, traits::BlakeTwo256, OpaqueExtrinsic};

use archive::{ArchiveCli, ArchiveError, ArchiveSystemBuilder};

/// The block number type used by Polkadot.
/// 32-bits will allow for 136 years of blocks assuming 1 block per second.
pub type BlockNumber = u32;
/// Header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Opaque, encoded, unchecked extrinsic.
pub type UncheckedExtrinsic = OpaqueExtrinsic;
/// Block type.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

native_executor_instance!(
    pub PolkadotExecutor,
    polkadot_runtime::api::dispatch,
    polkadot_runtime::native_version,
    frame_benchmarking::benchmarking::HostFunctions,
);

type PolkadotArchiveSystemBuilder =
    ArchiveSystemBuilder<Block, PolkadotExecutor, polkadot_runtime::RuntimeApi>;

fn main() -> Result<(), ArchiveError> {
    let config = ArchiveCli::init()?;
    log::info!("{:#?}", config);

    let archive = PolkadotArchiveSystemBuilder::with_config(config).build()?;
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
