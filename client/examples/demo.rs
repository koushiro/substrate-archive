use codec::Decode;
use sc_executor::{WasmExecutionMethod, WasmExecutor};
use sc_executor_common::error::WasmError;
use sp_core::{
    traits::{CallInWasmExt, MissingHostFunctions},
    Bytes, OpaqueMetadata,
};
use sp_externalities::ExternalitiesExt;
use sp_runtime_interface::with_externalities;
use sp_state_machine::BasicExternalities;
use sp_version::RuntimeVersion;
use sp_wasm_interface::HostFunctions;

fn main() {
    env_logger::init();

    let funcs = sp_io::SubstrateHostFunctions::host_functions()
        .into_iter()
        .collect::<Vec<_>>();
    let executor = WasmExecutor::new(WasmExecutionMethod::Interpreted, Some(128), funcs, 1, None);

    let code = include_bytes!("./polkadot_runtime-v29.compact.wasm");

    let version = runtime_version(executor.clone(), code);
    log::info!("Runtime Version: {:?}", version);

    let metadata = runtime_metadata(executor, code);
    let metadata = Bytes::from(metadata).0;
    log::info!("Runtime Metadata: {:?}", hex::encode(metadata.as_slice()));
}

fn runtime_version(executor: WasmExecutor, code: &[u8]) -> RuntimeVersion {
    let mut ext = BasicExternalities::default();
    ext.register_extension(CallInWasmExt::new(executor));

    let version = ext.execute_with(|| {
        with_externalities(|mut externalities| {
            externalities
                .extension::<CallInWasmExt>()
                .expect("No `CallInWasmExt` associated for the current context!")
                .call_in_wasm(
                    code,
                    None,
                    "Core_version",
                    &[],
                    &mut BasicExternalities::default(),
                    MissingHostFunctions::Allow,
                )
                .expect("called outside of an Externalities-provided environment")
        })
        .expect("wasm execution error")
    });
    decode_version(version).expect("decoder error")
}

fn decode_version(version: Vec<u8>) -> Result<RuntimeVersion, WasmError> {
    let v: RuntimeVersion = sp_api::OldRuntimeVersion::decode(&mut version.as_slice())
        .map_err(|_| {
            WasmError::Instantiation(
                "failed to decode \"Core_version\" result using old runtime version".into(),
            )
        })?
        .into();

    let core_api_id = sp_core::hashing::blake2_64(b"Core");
    if v.has_api_with(&core_api_id, |v| v >= 3) {
        RuntimeVersion::decode(&mut version.as_slice()).map_err(|_| {
            WasmError::Instantiation("failed to decode \"Core_version\" result".into())
        })
    } else {
        Ok(v)
    }
}

fn runtime_metadata(executor: WasmExecutor, code: &[u8]) -> OpaqueMetadata {
    let mut ext = BasicExternalities::default();
    ext.register_extension(CallInWasmExt::new(executor));

    let metadata = ext.execute_with(|| {
        with_externalities(|mut externalities| {
            externalities
                .extension::<CallInWasmExt>()
                .expect("No `CallInWasmExt` associated for the current context!")
                .call_in_wasm(
                    code,
                    None,
                    "Metadata_metadata",
                    &[],
                    &mut BasicExternalities::default(),
                    MissingHostFunctions::Allow,
                )
                .expect("called outside of an Externalities-provided environment")
        })
        .expect("wasm execution error")
    });
    OpaqueMetadata::decode(&mut metadata.as_slice()).expect("decode error")
}
