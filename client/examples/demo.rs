use std::{path::PathBuf, sync::Arc};

use codec::Decode;
use sc_client_api::BlockBackend;
use sc_executor::{WasmExecutionMethod, WasmExecutor};
use sc_executor_common::{error::WasmError, runtime_blob::RuntimeBlob};
use sp_core::{Bytes, OpaqueMetadata};
use sp_runtime::{
    generic::{self, BlockId},
    traits::BlakeTwo256,
    OpaqueExtrinsic,
};
use sp_state_machine::BasicExternalities;
use sp_version::RuntimeVersion;
use sp_wasm_interface::HostFunctions;

use archive_client::{ArchiveBackend, RocksDbConfig, SecondaryRocksDb};

/// The block number type used by Polkadot.
/// 32-bits will allow for 136 years of blocks assuming 1 block per second.
pub type BlockNumber = u32;
/// Header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Opaque, encoded, unchecked extrinsic.
pub type UncheckedExtrinsic = OpaqueExtrinsic;
/// Block type.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

fn main() {
    env_logger::init();

    let db = SecondaryRocksDb::open(RocksDbConfig {
        path: {
            let mut path = PathBuf::new();
            path.push("../../polkadot/database/chains/polkadot/db");
            path
        },
        cache_size: 128,
        secondary_db_path: {
            let mut path = PathBuf::new();
            path.push("./rocksdb_secondary");
            path
        },
    })
    .unwrap();
    let backend = ArchiveBackend::<Block>::new(Arc::new(db), Default::default()).unwrap();

    print_block(&backend, 512);
    print_block(&backend, 45568);

    print_runtime_info();
}

fn print_block(backend: &ArchiveBackend<Block>, n: u32) {
    let block = backend.block(&BlockId::Number(n)).unwrap();
    log::info!("Block #{}: {:?}", n, block);
    let justifications = backend.justifications(&BlockId::Number(n)).unwrap();
    log::info!("Block #{} Justifications: {:?}", n, justifications);
}

fn print_runtime_info() {
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

fn runtime_version(executor: WasmExecutor, wasm_code: &[u8]) -> RuntimeVersion {
    let mut ext = BasicExternalities::default();
    let version = executor
        .uncached_call(
            RuntimeBlob::uncompress_if_needed(wasm_code).unwrap(),
            &mut ext,
            true,
            "Core_version",
            &[],
        )
        .unwrap();
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

fn runtime_metadata(executor: WasmExecutor, wasm_code: &[u8]) -> OpaqueMetadata {
    let mut ext = BasicExternalities::default();
    let metadata = executor
        .uncached_call(
            RuntimeBlob::uncompress_if_needed(wasm_code).unwrap(),
            &mut ext,
            true,
            "Metadata_metadata",
            &[],
        )
        .unwrap();
    OpaqueMetadata::decode(&mut metadata.as_slice()).expect("decode error")
}
