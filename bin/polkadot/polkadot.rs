use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use serde::{Deserialize, Serialize};

use sc_chain_spec::{ChainSpec, ChainSpecExtension, GenericChainSpec};
use sc_client_api::{BadBlocks, ForkBlocks};
use sc_executor::native_executor_instance;

use archive::{Archive, ArchiveCli, ArchiveError, ArchiveSystemBuilder};
use archive_primitives::Block;

native_executor_instance!(
    pub PolkadotExecutor,
    polkadot_runtime::api::dispatch,
    polkadot_runtime::native_version,
    frame_benchmarking::benchmarking::HostFunctions,
);

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Clone, Default, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    /// Block numbers with known hashes.
    pub fork_blocks: ForkBlocks<Block>,
    /// Known bad block hashes.
    pub bad_blocks: BadBlocks<Block>,
}

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
