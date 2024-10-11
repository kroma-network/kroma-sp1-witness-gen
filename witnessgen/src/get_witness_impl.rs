use alloy_primitives::B256;
use serde::{Deserialize, Serialize};

use crate::{request_witness_impl::RequestResult, spec_impl::VKEY_HASH};

/// The result of a witness method.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WitnessResult {
    pub status: RequestResult,
    pub vkey_hash: B256,
    pub witness: Option<Vec<Vec<u8>>>,
}

impl Default for WitnessResult {
    fn default() -> Self {
        Self::new(RequestResult::None, None)
    }
}

impl WitnessResult {
    pub fn new(status: RequestResult, witness: Option<Vec<Vec<u8>>>) -> Self {
        Self { status, vkey_hash: *VKEY_HASH, witness }
    }

    pub fn size(&self) -> usize {
        match &self.witness {
            Some(witness) => witness.iter().map(|w| w.len()).sum(),
            None => 0,
        }
    }
}
