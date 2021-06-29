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
