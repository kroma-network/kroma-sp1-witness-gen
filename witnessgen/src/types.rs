use alloy_primitives::B256;
use serde::{Deserialize, Serialize};

use crate::{version::SP1_WITNESS_GEN_VERSION, PROGRAM_KEY};

// NOTE(Ethan): If the SP1 SDK version check is removed from the `Kroma validator`,
// delete the following code.
// const SP1_SDK_VERSION: &str = sp1_sdk::SP1_CIRCUIT_VERSION;
const SP1_SDK_VERSION: &str = "v4.0.0-rc.3";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpecResult {
    pub version: String,
    pub sp1_version: String,
    pub program_key: String,
}

impl SpecResult {
    pub fn new(version: String) -> Self {
        // NOTE(Ethan): We don’t want this v3 program key to be used
        // so we will temporarily set its value to an empty string.
        Self { version, sp1_version: SP1_SDK_VERSION.to_string(), program_key: "".to_string() }
    }
}

impl Default for SpecResult {
    fn default() -> Self {
        Self::new(SP1_WITNESS_GEN_VERSION.to_string())
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
    pub const EMPTY_WITNESS: Vec<Vec<u8>> = Vec::new();

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

#[derive(Clone, Debug, Default)]
pub struct TaskInfo {
    pub l2_hash: B256,
    pub l1_head_hash: B256,
}

impl TaskInfo {
    pub fn set(&mut self, l2_hash: B256, l1_head_hash: B256) {
        self.l2_hash = l2_hash;
        self.l1_head_hash = l1_head_hash;
    }

    pub fn release(&mut self) {
        self.l2_hash = B256::default();
        self.l1_head_hash = B256::default();
    }

    pub fn is_equal(&self, l2_hash: B256, l1_head_hash: B256) -> bool {
        self.l2_hash == l2_hash && self.l1_head_hash == l1_head_hash
    }

    pub fn is_empty(&self) -> bool {
        let default_value = B256::default();
        self.l2_hash == default_value && self.l1_head_hash == default_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vkey_hash() {
        let expected_vkey_hash =
            "0x009be3ffcfcbc348e96d5b5a23294c7afa2f67b812821bd1bada13a6026c19b1";
        assert_eq!(PROGRAM_KEY.to_string(), expected_vkey_hash);
    }
}
