use kroma_utils::deps_version::SP1_SDK_VERSION;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sp1_sdk::{HashableKey, ProverClient};

pub const SINGLE_BLOCK_ELF: &[u8] = include_bytes!("../../program/elf/fault-proof-elf");
pub static PROGRAM_KEY: Lazy<String> = Lazy::new(|| {
    let prover = ProverClient::new();
    let (_, vkey) = prover.setup(SINGLE_BLOCK_ELF);
    vkey.bytes32()
});

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpecResult {
    pub version: String,
    pub sp1_version: String,
    pub program_key: String,
}

impl SpecResult {
    pub fn new(version: String) -> Self {
        Self { version, sp1_version: SP1_SDK_VERSION.to_string(), program_key: PROGRAM_KEY.to_string() }
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
    pub program_key: String,
    pub witness: String,
}

impl Default for WitnessResult {
    fn default() -> Self {
        Self::new(RequestResult::None, "".to_string())
    }
}

impl WitnessResult {
    pub fn new<T: ToString>(status: RequestResult, witness: T) -> Self {
        Self { status, program_key: PROGRAM_KEY.to_string(), witness: witness.to_string() }
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

    pub fn string_to_witness_buf(witness: &str) -> Vec<Vec<u8>> {
        let witness = hex::decode(witness.strip_prefix("0x").unwrap()).unwrap();
        bincode::deserialize(&witness).unwrap()
    }

    pub fn get_witness_buf(&self) -> Vec<Vec<u8>> {
        Self::string_to_witness_buf(&self.witness)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vkey_hash() {
        let expected_vkey_hash =
            "0x0057b1a16832ad3f0321ac2a568d6dee0638f9cd772f1488d10b6a911f4a1b68";
        assert_eq!(PROGRAM_KEY.to_string(), expected_vkey_hash);
    }
}
