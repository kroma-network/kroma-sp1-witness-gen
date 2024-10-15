use alloy_primitives::B256;
use serde::{Deserialize, Serialize};

use crate::request_prove_impl::RequestResult;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProofResult {
    pub request_status: RequestResult,
    pub public_values: Vec<u8>,
    pub proof: Vec<u8>,
    pub vkey_hash: B256,
}

impl ProofResult {
    // pub fn new(
    //     request_status: RequestResult,
    //     public_values: Vec<u8>,
    //     proof: Vec<u8>,
    //     vkey_hash: B256,
    // ) -> Self {
    //     Self { request_status, public_values, proof, vkey_hash: *VKEY_HASH }
    // }
    pub fn new(request_status: RequestResult, proof: Vec<u8>) -> Self {
        Self { request_status, public_values: vec![], proof, vkey_hash: B256::default() }
    }
}
