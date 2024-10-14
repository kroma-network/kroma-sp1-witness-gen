use alloy_primitives::B256;
use serde::{Deserialize, Serialize};

use crate::{request_witness_impl::RequestResult, spec_impl::VKEY_HASH};

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

    pub fn new_from_bytes(status: RequestResult, witness: Vec<Vec<u8>>) -> Self {
        let serialized_witness = bincode::serialize(&witness).unwrap();
        let hex_encoded_with_prefix = "0x".to_string() + hex::encode(&serialized_witness).as_ref();
        Self::new(status, hex_encoded_with_prefix)
    }

    pub fn size(&self) -> usize {
        hex::decode(&self.witness).unwrap().len()
    }

    pub fn get_witness(&mut self) -> Vec<Vec<u8>> {
        let witness = hex::decode(&self.witness.strip_prefix("0x").unwrap()).unwrap();
        bincode::deserialize(&witness).unwrap()
    }
}
