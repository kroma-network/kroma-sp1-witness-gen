use kroma_common::PROGRAM_KEY;
use kroma_common::{deps_version::SP1_SDK_VERSION, version::KROMA_VERSION};
use serde::{Deserialize, Serialize};
use sp1_sdk::SP1ProofWithPublicValues;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpecResult {
    pub version: String,
    pub sp1_version: String,
    pub program_key: String,
}

impl SpecResult {
    pub fn new(version: String, sp1_version: String, program_key: String) -> Self {
        Self { version, sp1_version, program_key }
    }
}

impl Default for SpecResult {
    fn default() -> Self {
        SpecResult::new(
            KROMA_VERSION.to_string(),
            SP1_SDK_VERSION.to_string(),
            PROGRAM_KEY.to_string(),
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RequestResult {
    None,
    Processing,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProofResult {
    pub request_id: String,
    pub request_status: RequestResult,
    pub program_key: String,
    pub public_values: String,
    pub proof: String,
}

impl ProofResult {
    pub fn new<T: ToString>(
        request_id: &T,
        request_status: RequestResult,
        proof: SP1ProofWithPublicValues,
    ) -> Self {
        Self {
            request_id: request_id.to_string(),
            request_status,
            program_key: PROGRAM_KEY.to_string(),
            public_values: hex::encode(&proof.public_values),
            proof: hex::encode(proof.bytes()),
        }
    }

    pub fn none() -> Self {
        Self {
            request_id: "".to_string(),
            request_status: RequestResult::None,
            program_key: PROGRAM_KEY.to_string(),
            public_values: "".to_string(),
            proof: "".to_string(),
        }
    }

    pub fn processing(request_id: String) -> Self {
        Self {
            request_id,
            request_status: RequestResult::Processing,
            program_key: PROGRAM_KEY.to_string(),
            public_values: "".to_string(),
            proof: "".to_string(),
        }
    }

    pub fn failed(request_id: String) -> Self {
        Self {
            request_id,
            request_status: RequestResult::Failed,
            program_key: PROGRAM_KEY.to_string(),
            public_values: "".to_string(),
            proof: "".to_string(),
        }
    }
}
