use crate::utils::u32_to_u8;
use alloy_primitives::B256;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sp1_sdk::{HashableKey, ProverClient};

const SINGLE_BLOCK_ELF: &[u8] = include_bytes!("../../program/elf/fault-proof-elf");
pub static VKEY_HASH: Lazy<B256> = Lazy::new(|| {
    let prover = ProverClient::new();
    let (_, vkey) = prover.setup(SINGLE_BLOCK_ELF);
    B256::from(u32_to_u8(vkey.vk.hash_u32()))
});

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpecResult {
    pub version: String,
    pub vkey_hash: B256,
}

impl SpecResult {
    pub fn new(version: String) -> Self {
        Self { version, vkey_hash: *VKEY_HASH }
    }
}

impl Default for SpecResult {
    fn default() -> Self {
        // TODO: handle versioning more flexibly.
        Self::new("0.1.0".to_string())
    }
}

pub fn spec_impl() -> SpecResult {
    let spec = SpecResult::default();
    tracing::info_span!("spec").in_scope(|| {
        tracing::info!("return the specification:\n{:#?}", spec);
    });
    spec
}

#[cfg(test)]
mod tests {
    use alloy_primitives::hex::FromHex;

    use super::*;

    #[test]
    fn test_vkey_hash() {
        let expected_vkey_hash =
            B256::from_hex("0x6c15e3bb696329c15f6b963e40ac4c3841e726ef6fcaea042daf4b3d056e8d2f")
                .unwrap();
        assert_eq!(*VKEY_HASH, expected_vkey_hash);
    }
}
