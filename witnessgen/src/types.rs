use alloy_primitives::B256;
use kroma_utils::{deps_version::SP1_SDK_VERSION, utils::u32_to_u8};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sp1_sdk::{HashableKey, ProverClient};

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

impl SpecResult {
    pub fn new(version: String) -> Self {
        Self { version, sp1_version: SP1_SDK_VERSION.to_string(), vkey_hash: *VKEY_HASH }
    }
}

impl Default for SpecResult {
    fn default() -> Self {
        // TODO: handle versioning more flexibly.
        Self::new("0.1.0".to_string())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RequestResult {
    None,
    Processing,
    Completed,
    Failed,
}

/// The result of a witness method.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WitnessResult {
    pub status: RequestResult,
    pub vkey_hash: B256,
    pub witness: String,
}

impl Default for WitnessResult {
    fn default() -> Self {
        Self::new(RequestResult::None, "".to_string())
    }
}

impl WitnessResult {
    pub fn new<T: ToString>(status: RequestResult, witness: T) -> Self {
        Self { status, vkey_hash: *VKEY_HASH, witness: witness.to_string() }
    }

    pub fn new_with_status(status: RequestResult) -> Self {
        Self::new(status, "".to_string())
    }

    // Note(Ethan): `sp1-core-machine::SP1Stdin` has witness as `Vec<Vec<u8>>`.
    pub fn new_from_witness_buf(status: RequestResult, buf: Vec<Vec<u8>>) -> Self {
        let serialized_witness = bincode::serialize(&buf).unwrap();
        let hex_encoded_with_prefix = "0x".to_string() + hex::encode(&serialized_witness).as_ref();
        Self::new(status, hex_encoded_with_prefix)
    }

    pub fn string_to_witness_buf(witness: &String) -> Vec<Vec<u8>> {
        let witness = hex::decode(&witness.strip_prefix("0x").unwrap()).unwrap();
        bincode::deserialize(&witness).unwrap()
    }

    pub fn get_witness_buf(&self) -> Vec<Vec<u8>> {
        Self::string_to_witness_buf(&self.witness)
    }
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
