use alloy_primitives::B256;
use anyhow::{anyhow, Result};
use kroma_zkvm_common::db::FileDB;
use sp1_sdk::SP1ProofWithPublicValues;

static CAPACITY: usize = 1000;
static VALUE_EXPIRING_SECS: usize = 7 * 24 * 60 * 60; // 604800; 7 days in seconds.

pub struct ProofDB {
    db: FileDB,
}

impl ProofDB {
    pub fn new(db_file_path: &str) -> Self {
        Self { db: FileDB::new(db_file_path.into(), CAPACITY, VALUE_EXPIRING_SECS) }
    }

    fn build_key(l2_hash: &B256, l1_head_hash: &B256) -> Vec<u8> {
        let mut key = Vec::with_capacity(64);
        key.extend_from_slice(l2_hash.as_slice());
        key.extend_from_slice(l1_head_hash.as_slice());
        key
    }

    fn convert_req_id_as_key<T: ToString>(request_id: &T) -> Vec<u8> {
        bincode::serialize(&request_id.to_string())
            .map_err(|e| anyhow!("Failed to serialize value: {}", e))
            .unwrap()
    }

    pub fn set_request_id(
        &self,
        l2_hash: &B256,
        l1_head_hash: &B256,
        request_id: &String,
    ) -> Result<()> {
        let key = Self::build_key(l2_hash, l1_head_hash);
        self.db.set(&key, &request_id).map_err(|e| anyhow!("Failed to set request id: {}", e))
    }

    pub fn set_proof<T: ToString>(
        &self,
        request_id: &T,
        proof: &SP1ProofWithPublicValues,
    ) -> Result<()> {
        let req_id_key = Self::convert_req_id_as_key(request_id);
        self.db.set(&req_id_key, &proof)
    }

    pub fn get_request_id(&self, l2_hash: &B256, l1_head_hash: &B256) -> Option<String> {
        let key = Self::build_key(l2_hash, l1_head_hash);
        self.db.get(&key)
    }

    pub fn get_proof(
        &self,
        l2_hash: &B256,
        l1_head_hash: &B256,
    ) -> Option<SP1ProofWithPublicValues> {
        let request_id = self.get_request_id(l2_hash, l1_head_hash);
        request_id.as_ref()?;
        self.get_proof_by_id(&request_id.unwrap())
    }

    pub fn get_proof_by_id(&self, request_id: &str) -> Option<SP1ProofWithPublicValues> {
        let req_id_key = Self::convert_req_id_as_key(&request_id);
        self.db.get(&req_id_key)
    }
}
