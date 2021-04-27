use serde::{Deserialize, Serialize};

use sc_chain_spec::ChainSpecExtension;
use sc_client_api::{BadBlocks, ForkBlocks};
use sp_runtime::{generic, traits::BlakeTwo256, OpaqueExtrinsic};

/// The block number type used by Polkadot.
/// 32-bits will allow for 136 years of blocks assuming 1 block per second.
pub type BlockNumber = u32;
/// Header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Opaque, encoded, unchecked extrinsic.
pub type UncheckedExtrinsic = OpaqueExtrinsic;
/// Block type.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

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
