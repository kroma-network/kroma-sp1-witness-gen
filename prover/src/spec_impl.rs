use alloy_primitives::B256;
use kroma_utils::utils::u32_to_u8;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sp1_sdk::{HashableKey, ProverClient};

// TODO: integrate elf and vkey_hash
pub const SINGLE_BLOCK_ELF: &[u8] = include_bytes!("../../program/elf/fault-proof-elf");
pub static VKEY_HASH: Lazy<B256> = Lazy::new(|| {
    let prover = ProverClient::new();
    let (_, vkey) = prover.setup(SINGLE_BLOCK_ELF);
    B256::from(u32_to_u8(vkey.vk.hash_u32()))
});

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpecResult {
    pub version: String,
    pub sp1_version: String,
    pub vkey_hash: B256,
}

impl Default for SpecResult {
    fn default() -> Self {
        SpecResult {
            version: "0.1.0".to_string(),
            sp1_version: "".to_string(),
            vkey_hash: *VKEY_HASH,
        }
    }
}
