use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use sc_chain_spec::{ChainSpec, GenericChainSpec};
use sc_executor::native_executor_instance;

use archive::{ArchiveCli, ArchiveError, ArchiveSystemBuilder, Block, Extensions};

native_executor_instance!(
    pub KusamaExecutor,
    kusama_runtime::api::dispatch,
    kusama_runtime::native_version,
    frame_benchmarking::benchmarking::HostFunctions,
);

/// The `ChainSpec` parametrised for the kusama runtime.
type KusamaChainSpec = GenericChainSpec<kusama_runtime::GenesisConfig, Extensions>;

type KusamaArchiveSystemBuilder =
    ArchiveSystemBuilder<Block, KusamaExecutor, kusama_runtime::RuntimeApi>;

fn main() -> Result<(), ArchiveError> {
    let config = ArchiveCli::init()?;
    log::info!(target: "archive", "{:#?}", config);

    let chain_spec = KusamaChainSpec::from_json_bytes(&include_bytes!("./kusama.json")[..])
        .expect("generate chain spec from json file");
    let genesis = chain_spec.as_storage_builder();

    let archive = KusamaArchiveSystemBuilder::with_config(config).build(genesis)?;
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
