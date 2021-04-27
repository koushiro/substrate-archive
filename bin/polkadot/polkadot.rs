use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use sc_chain_spec::{ChainSpec, GenericChainSpec};
use sc_executor::native_executor_instance;

use archive::{ArchiveCli, ArchiveError, ArchiveSystemBuilder, Block, Extensions};

native_executor_instance!(
    pub PolkadotExecutor,
    polkadot_runtime::api::dispatch,
    polkadot_runtime::native_version,
    frame_benchmarking::benchmarking::HostFunctions,
);

/// The `ChainSpec` parametrised for the polkadot runtime.
type PolkadotChainSpec = GenericChainSpec<polkadot_runtime::GenesisConfig, Extensions>;

type PolkadotArchiveSystemBuilder =
    ArchiveSystemBuilder<Block, PolkadotExecutor, polkadot_runtime::RuntimeApi>;

fn main() -> Result<(), ArchiveError> {
    let config = ArchiveCli::init()?;
    log::info!(target: "archive", "{:#?}", config);

    let chain_spec = PolkadotChainSpec::from_json_bytes(&include_bytes!("./polkadot.json")[..])
        .expect("generate chain spec from json bytes");
    let genesis = chain_spec.as_storage_builder();

    let archive = PolkadotArchiveSystemBuilder::with_config(config).build(genesis)?;
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
