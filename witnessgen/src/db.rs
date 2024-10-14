use alloy_primitives::B256;
use anyhow::{anyhow, Result};
use rocksdb::{Options, DB};
use std::path::PathBuf;

/// A simple, synchronous key-value store that stores data on disk.
pub struct WitnessStore {
    store_path: PathBuf,
    db: DB,
}

impl WitnessStore {
    /// Create a new [WitnessStore] with the given data directory.
    pub fn new(store_path: PathBuf) -> Self {
        let db = DB::open(&Self::get_db_options(), store_path.as_path())
            .unwrap_or_else(|e| panic!("Failed to open database at {store_path:?}: {e}"));

        Self { store_path, db }
    }

    /// Gets the [Options] for the underlying RocksDB instance.
    fn get_db_options() -> Options {
        let mut options = Options::default();
        options.set_compression_type(rocksdb::DBCompressionType::Snappy);
        options.create_if_missing(true);
        options
    }

    fn build_key(l2_hash: B256, l1_head_hash: B256) -> Vec<u8> {
        let mut key = Vec::with_capacity(64);
        key.extend_from_slice(&l2_hash.as_slice());
        key.extend_from_slice(&l1_head_hash.as_slice());
        key
    }

    pub fn get(&self, l2_hash: B256, l1_head_hash: B256) -> Result<Vec<Vec<u8>>> {
        let key = Self::build_key(l2_hash, l1_head_hash);
        let result = self.db.get(key);
        match result {
            Ok(Some(value)) => {
                let witness: Vec<Vec<u8>> = bincode::deserialize(&value)
                    .map_err(|e| anyhow!("Failed to deserialize value: {}", e))?;
                Ok(witness)
            }
            Ok(None) => Err(anyhow!("Key not found")),
            Err(e) => Err(anyhow!("Failed to get value: {}", e)),
        }
    }

    pub fn set(&self, l2_hash: B256, l1_head_hash: B256, witness: Vec<Vec<u8>>) -> Result<()> {
        let key = Self::build_key(l2_hash, l1_head_hash);
        let serialized_witness = bincode::serialize(&witness)
            .map_err(|e| anyhow!("Failed to serialize value: {}", e))?;
        self.db
            .put(key, serialized_witness)
            .map_err(|e| anyhow!("Failed to set key-value pair: {}", e))
    }
}

impl Drop for WitnessStore {
    fn drop(&mut self) {
        let _ = DB::destroy(&Self::get_db_options(), self.store_path.as_path());
    }
}

#[cfg(test)]
mod tests {
    use crate::WitnessResult;

    use super::*;

    use alloy_primitives::b256;
    use ctor::dtor;
    use once_cell::sync::Lazy;
    use std::{fs, sync::Mutex};

    static TEST_WITNESS_STORE_DIR: &str = "/tmp/witness_store";
    static WITNESS_STORE: Lazy<Mutex<WitnessStore>> =
        Lazy::new(|| Mutex::new(WitnessStore::new(PathBuf::from(TEST_WITNESS_STORE_DIR))));

    #[dtor]
    fn teardown() {
        fs::remove_dir_all(TEST_WITNESS_STORE_DIR).unwrap()
    }

    #[test]
    fn test_witness_store() {
        let store = WITNESS_STORE.lock().unwrap();
        let mut witness_result =
            WitnessResult::new_from_bytes(crate::RequestResult::Completed, vec![vec![1, 2, 3]]);

        let l2_hash = b256!("0000000000000000000000000000000000000000000000000000000000000001");
        let l1_head_hash =
            b256!("0000000000000000000000000000000000000000000000000000000000000001");

        store.set(l2_hash, l1_head_hash, witness_result.get_witness()).unwrap();
        let result = store.get(l2_hash, l1_head_hash).unwrap();
        assert_eq!(witness_result.get_witness(), result);

        let l2_hash = b256!("0000000000000000000000000000000000000000000000000000000000000002");
        let l1_head_hash =
            b256!("0000000000000000000000000000000000000000000000000000000000000002");
        let result = store.get(l2_hash, l1_head_hash);
        assert!(result.is_err());
    }
}
