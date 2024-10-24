use alloy_primitives::B256;
use kroma_utils::{deps_version::SP1_SDK_VERSION, utils::u32_to_u8};
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

impl SpecResult {
    pub fn new(version: String, sp1_version: String, vkey_hash: B256) -> Self {
        Self { version, sp1_version, vkey_hash }
    }
}

impl Default for SpecResult {
    fn default() -> Self {
        SpecResult::new("0.1.0".to_string(), SP1_SDK_VERSION.to_string(), *VKEY_HASH)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RequestResult {
    None,
    Processing,
    Completed,
    Failed(String),
}

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

    pub fn processing(request_id: String) -> Self {
        Self {
            request_id,
            request_status: RequestResult::Processing,
            vkey_hash: *VKEY_HASH,
            public_values: "".to_string(),
            proof: "".to_string(),
        }
    }

    pub fn failed(request_id: String, message: String) -> Self {
        Self {
            request_id,
            request_status: RequestResult::Failed(message),
            vkey_hash: *VKEY_HASH,
            public_values: "".to_string(),
            proof: "".to_string(),
        }
    }
}
