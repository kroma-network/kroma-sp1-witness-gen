use anyhow::{anyhow, Result};
use rocksdb::{Options, DB};
use serde::{de::DeserializeOwned, Serialize};
use std::path::PathBuf;

/// A simple, synchronous key-value store that stores data on disk.
pub struct FileDB {
    db_file_path: PathBuf,
    db: DB,
}

impl FileDB {
    /// Create a new [WitnessStore] with the given data directory.
    pub fn new(db_file_path: PathBuf) -> Self {
        let db = DB::open(&Self::get_db_options(), db_file_path.as_path())
            .unwrap_or_else(|e| panic!("Failed to open database at {db_file_path:?}: {e}"));

        Self { db_file_path, db }
    }

    /// Gets the [Options] for the underlying RocksDB instance.
    fn get_db_options() -> Options {
        let mut options = Options::default();
        options.set_compression_type(rocksdb::DBCompressionType::Snappy);
        options.create_if_missing(true);
        options
    }

    fn append_timestamp(value: Vec<u8>) -> Vec<u8> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            .to_le_bytes();
        let mut updated_vector = Vec::with_capacity(value.len() + timestamp.len());
        updated_vector.extend_from_slice(&timestamp);
        updated_vector.extend_from_slice(&value);
        updated_vector
    }

    pub fn set<T: Serialize>(&self, key: &Vec<u8>, value: &T) -> Result<()> {
        let serialized_value =
            bincode::serialize(value).map_err(|e| anyhow!("Failed to serialize value: {}", e))?;

        // Append `timestamp` to the serialized value.
        let time_value = Self::append_timestamp(serialized_value);

        // Store the value with `timestamp` in the database.
        self.db.put(key, time_value).map_err(|e| anyhow!("Failed to set key-value pair: {}", e))
    }

    fn split_value_and_timestamp(mut value_from_db: Vec<u8>) -> (u64, Vec<u8>) {
        let timestamp_bytes: Vec<u8> = value_from_db.drain(..8).collect();
        let timestamp = u64::from_le_bytes(timestamp_bytes.try_into().unwrap());
        (timestamp, value_from_db)
    }

    pub fn get_with_timestamp<T: DeserializeOwned>(&self, key: &Vec<u8>) -> Option<(u64, T)> {
        let result = self.db.get(key);

        // Fetch the value from the database.
        let value_from_db = match result {
            Ok(Some(value)) => value,
            Ok(None) => return None,
            Err(e) => {
                tracing::error!("Unexpected error occurs in db: {:?}", e);
                return None;
            }
        };

        // split the `timestamp` and `value` from the fetched value.
        let (timestamp, serialized_value) = Self::split_value_and_timestamp(value_from_db);
        let value: T = bincode::deserialize(&serialized_value)
            .map_err(|e| anyhow!("Failed to deserialize value: {}", e))
            .unwrap();
        Some((timestamp, value))
    }

    pub fn get<T: DeserializeOwned>(&self, key: &Vec<u8>) -> Option<T> {
        self.get_with_timestamp(key).map(|(_, value)| value)
    }
}

impl Drop for FileDB {
    fn drop(&mut self) {
        let _ = DB::destroy(&Self::get_db_options(), self.db_file_path.as_path());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ctor::dtor;
    use once_cell::sync::Lazy;
    use std::{fs, sync::Mutex};

    static TEST_STORE_DIR: &str = "/tmp/store";
    static STORE: Lazy<Mutex<FileDB>> =
        Lazy::new(|| Mutex::new(FileDB::new(PathBuf::from(TEST_STORE_DIR))));

    #[dtor]
    fn teardown() {
        fs::remove_dir_all(TEST_STORE_DIR).unwrap()
    }

    #[test]
    fn test_store() {
        let store = STORE.lock().unwrap();
        let value = vec![vec![1, 2, 3]];

        // key1
        let key1 = vec![0, 0, 0, 1];
        store.set(&key1, &value).unwrap();

        let result: Vec<Vec<u8>> = store.get::<Vec<Vec<u8>>>(&key1).unwrap();
        assert_eq!(value, result);

        let key2 = vec![0, 0, 0, 2];
        let result = store.get::<Vec<Vec<u8>>>(&key2);
        assert!(result.is_none());
    }
}
