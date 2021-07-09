use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use sc_chain_spec::ChainSpec;

use crate::database::RocksDbConfig;

/// Client configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Rocksdb configuration.
    pub rocksdb: RocksDbConfig,

    /// Executor configuration.
    pub executor: ExecutorConfig,

    /// Should offchain workers be executed.
    #[serde(skip)]
    pub offchain_worker: OffchainWorkerConfig,
    /// Directory where local WASM runtimes live. These runtimes take precedence
    /// over on-chain runtimes when the spec version matches. Set to `None` to
    /// disable overrides (default).
    pub wasm_runtime_overrides: Option<PathBuf>,
    /// Map of WASM runtime substitute starting at the child of the given block until the runtime
    /// version doesn't match anymore.
    ///
    /// NOTE: Not to be confused with 'wasm_runtime_overrides'. code_substitutes
    /// are included in the chain_spec and primarily for fixing problematic on-chain wasm.
    /// If both are in use, the `wasm_runtime_overrides` takes precedence.
    #[serde(skip)]
    pub(crate) wasm_runtime_substitutes: HashMap<String, Vec<u8>>,
}

impl ClientConfig {
    /// Set the code substitutes for a chain.
    pub fn set_code_substitutes(&mut self, spec: &dyn ChainSpec) {
        self.wasm_runtime_substitutes = spec.code_substitutes();
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutorConfig {
    /// Wasm execution method.
    pub wasm_exec_method: WasmExecutionMethod,
    /// The default number of 64KB pages to allocate for Wasm execution
    pub default_heap_pages: Option<u64>,
    /// The size of the instances cache.
    ///
    /// The default value is 8.
    #[serde(default = "default_max_runtime_instances")]
    pub max_runtime_instances: usize,
}

/// Specification of different methods of executing the runtime Wasm code.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WasmExecutionMethod {
    /// Uses the Wasmi interpreter.
    Interpreted,
    /// Uses the Wasmtime compiled runtime.
    Compiled,
}

impl Default for WasmExecutionMethod {
    fn default() -> WasmExecutionMethod {
        WasmExecutionMethod::Interpreted
    }
}

impl From<WasmExecutionMethod> for sc_executor::WasmExecutionMethod {
    fn from(method: WasmExecutionMethod) -> Self {
        match method {
            WasmExecutionMethod::Interpreted => Self::Interpreted,
            WasmExecutionMethod::Compiled => Self::Compiled,
        }
    }
}

const fn default_max_runtime_instances() -> usize {
    8
}

/// Configuration of the database of the client.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct OffchainWorkerConfig {
    /// If this is allowed.
    pub enabled: bool,
    /// allow writes from the runtime to the offchain worker database.
    pub indexing_enabled: bool,
}
