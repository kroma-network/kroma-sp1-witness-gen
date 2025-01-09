use alloy_primitives::B256;
use anyhow::{anyhow, Result};
use kroma_common::db::FileDB;

static CAPACITY: usize = 10;
static VALUE_EXPIRING_SECS: usize = 24 * 60 * 60; // 86400; A day in seconds.

pub struct WitnessDB {
    db: FileDB,
}

impl WitnessDB {
    pub fn new(db_file_path: &str) -> Self {
        Self { db: FileDB::new(db_file_path.into(), CAPACITY, VALUE_EXPIRING_SECS) }
    }

    fn build_key(l2_hash: &B256, l1_head_hash: &B256) -> Vec<u8> {
        let mut key = Vec::with_capacity(64);
        key.extend_from_slice(l2_hash.as_slice());
        key.extend_from_slice(l1_head_hash.as_slice());
        key
    }

    pub fn get(&self, l2_hash: &B256, l1_head_hash: &B256) -> Option<Vec<Vec<u8>>> {
        let key = Self::build_key(l2_hash, l1_head_hash);
        self.db.get(&key)
    }

    pub fn set(
        &self,
        l2_hash: &B256,
        l1_head_hash: &B256,
        witness_buf: Vec<Vec<u8>>,
    ) -> Result<()> {
        let key = Self::build_key(l2_hash, l1_head_hash);
        self.db.set(&key, &witness_buf).map_err(|e| anyhow!("Failed to set witness: {}", e))
    }

    pub fn remove(&self, l2_hash: &B256, l1_head_hash: &B256) -> Result<()> {
        let key = Self::build_key(l2_hash, l1_head_hash);
        self.db.remove(&key).map_err(|e| anyhow!("Failed to remove witness: {}", e))
    }
}
