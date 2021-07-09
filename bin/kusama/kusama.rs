use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use serde::{Deserialize, Serialize};

use sc_chain_spec::{ChainSpecExtension, GenericChainSpec};
use sc_client_api::{BadBlocks, ForkBlocks};
use sc_executor::native_executor_instance;

use archive::{Archive, ArchiveCli, ArchiveError, ArchiveSystemBuilder};
use archive_primitives::Block;

native_executor_instance!(
    pub KusamaExecutor,
    kusama_runtime::api::dispatch,
    kusama_runtime::native_version,
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

/// The `ChainSpec` parametrised for the kusama runtime.
type KusamaChainSpec = GenericChainSpec<kusama_runtime::GenesisConfig, Extensions>;

type KusamaArchiveSystemBuilder =
    ArchiveSystemBuilder<Block, KusamaExecutor, kusama_runtime::RuntimeApi>;

fn main() -> Result<(), ArchiveError> {
    let config = ArchiveCli::init()?;
    log::info!(target: "archive", "{:#?}", config);

    let chain_spec = KusamaChainSpec::from_json_bytes(&include_bytes!("./kusama.json")[..])
        .expect("generate chain spec from json bytes");
    let archive = KusamaArchiveSystemBuilder::with_config(config).build(&chain_spec)?;
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
