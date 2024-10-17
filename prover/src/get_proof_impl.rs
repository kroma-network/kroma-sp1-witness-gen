use alloy_primitives::B256;
use serde::{Deserialize, Serialize};

use crate::{request_prove_impl::RequestResult, spec_impl::VKEY_HASH};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProofResult {
    pub request_id: String,
    pub request_status: RequestResult,
    pub vkey_hash: B256,
    pub public_values: String,
    pub proof: String,
}

impl ProofResult {
    pub fn new<T: ToString>(
        request_id: &T,
        request_status: RequestResult,
        public_values: T,
        proof: T,
    ) -> Self {
        Self {
            request_id: request_id.to_string(),
            request_status,
            vkey_hash: *VKEY_HASH,
            public_values: public_values.to_string(),
            proof: proof.to_string(),
        }
    }

    pub fn none() -> Self {
        Self {
            request_id: "".to_string(),
            request_status: RequestResult::None,
            vkey_hash: *VKEY_HASH,
            public_values: "".to_string(),
            proof: "".to_string(),
        }
    }

    pub fn unexpected(request_id: String) -> Self {
        Self {
            request_id,
            request_status: RequestResult::UnexpectedError("Unexpected error".to_string()),
            vkey_hash: *VKEY_HASH,
            public_values: "".to_string(),
            proof: "".to_string(),
        }
    }

    pub fn processing(request_id: String) -> Self {
        Self {
            request_id,
            request_status: RequestResult::Processing,
            vkey_hash: *VKEY_HASH,
            public_values: "".to_string(),
            proof: "".to_string(),
        }
    }
}
