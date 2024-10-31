use anyhow::{anyhow, Result};
use rocksdb::{Options, DB};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::RwLock;
use std::vec;
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

static KEY_FOR_SIZE: [u8; 4] = [0, 0, 0, 0];

/// A simple, synchronous key-value store that stores data on disk.
pub struct FileDB {
    db_file_path: PathBuf,
    db: DB,
    size: RwLock<usize>,
    capacity: usize,
    expiring_secs: usize,
}

impl FileDB {
    /// Create a new [WitnessStore] with the given data directory.
    pub fn new(db_file_path: PathBuf, capacity: usize, expiring_secs: usize) -> Self {
        let db = DB::open(&Self::get_db_options(), db_file_path.as_path())
            .unwrap_or_else(|e| panic!("Failed to open database at {db_file_path:?}: {e}"));

        let size = Self::get_size(&db);
        tracing::info!("Initiated with a db file containing {size} items.");

        Self { db_file_path, db, size: RwLock::new(size), capacity, expiring_secs }
    }

    /// Gets the [Options] for the underlying RocksDB instance.
    fn get_db_options() -> Options {
        let mut options = Options::default();
        options.set_compression_type(rocksdb::DBCompressionType::Snappy);
        options.create_if_missing(true);
        options
    }

    fn set_size(db: &DB, num: usize) {
        db.put(KEY_FOR_SIZE, num.to_le_bytes())
            .map_err(|e| anyhow!("Failed to set key-value pair: {}", e))
            .unwrap();
    }

    fn get_size(db: &DB) -> usize {
        match db.get(KEY_FOR_SIZE).unwrap() {
            Some(value) => usize::from_le_bytes(value.try_into().unwrap()),
            None => {
                Self::set_size(db, 0);
                0
            }
        }
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

    fn split_value_and_timestamp(mut value_from_db: Vec<u8>) -> (u64, Vec<u8>) {
        let timestamp_bytes: Vec<u8> = value_from_db.drain(..8).collect();
        let timestamp = u64::from_le_bytes(timestamp_bytes.try_into().unwrap());
        (timestamp, value_from_db)
    }

    // Ensure that the number of items in the database is within the capacity.
    fn compact(&self, mut size: usize) -> usize {
        // Determine from which point you need to delete the value.
        let timestamp =
            SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs();
        let limit = timestamp - self.expiring_secs as u64;

        let mut oldest_key: Vec<u8> = vec![];
        let mut oldest_timestamp = u64::MAX;

        let mut db_iter = self.db.full_iterator(rocksdb::IteratorMode::Start);
        while let Some(Ok((key, value))) = db_iter.next() {
            if key.to_vec() == KEY_FOR_SIZE.to_vec() {
                continue;
            }

            let (timestamp, _) = Self::split_value_and_timestamp(value.to_vec());
            if timestamp < oldest_timestamp {
                oldest_timestamp = timestamp;
                oldest_key = key.to_vec();
            }
            if timestamp < limit {
                self.db.delete(key).unwrap();
                size -= 1;
            }
        }

        if size >= self.capacity {
            self.db.delete(oldest_key).unwrap();
            size -= 1;
        }

        size
    }

    pub fn set<T: Serialize>(&self, key: &Vec<u8>, value: &T) -> Result<()> {
        let mut size = self.size.write().unwrap();

        if *size >= self.capacity {
            *size = self.compact(*size);
            Self::set_size(&self.db, *size);
        }

        if self.get::<Vec<u8>>(key).is_none() {
            // Increment the number of items. And set the number of items in the database.
            *size += 1;
            Self::set_size(&self.db, *size);
        }

        // Store value after serializing and appending `timestamp`.
        let serialized_value =
            bincode::serialize(value).map_err(|e| anyhow!("Failed to serialize value: {}", e))?;
        let time_value = Self::append_timestamp(serialized_value);
        let ret = self.db.put(key, time_value);

        ret.map_err(|e| anyhow!("Failed to set key-value pair: {}", e))
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
    use std::{fs, thread, time::Duration};

    use super::*;

    #[test]
    fn test_store() {
        let store_dir = "/tmp/store";
        let store = FileDB::new(PathBuf::from(store_dir), 10, 10);
        let value = vec![vec![1, 2, 3]];

        // key1
        let key1 = vec![0, 0, 0, 1];
        store.set(&key1, &value).unwrap();

        let result: Vec<Vec<u8>> = store.get::<Vec<Vec<u8>>>(&key1).unwrap();
        assert_eq!(value, result);

        let key2 = vec![0, 0, 0, 2];
        let result = store.get::<Vec<Vec<u8>>>(&key2);
        assert!(result.is_none());

        drop(store);
        fs::remove_dir_all(store_dir).unwrap()
    }

    fn get_store_size(db: &FileDB) -> usize {
        let size = db.size.write().unwrap();
        *size
    }

    #[test]
    fn test_compaction() {
        let store_dir = "/tmp/store-compaction";
        let store = FileDB::new(PathBuf::from(store_dir), 5, 1);
        let value = vec![vec![1, 2, 3]];

        // Initial check.
        let size = get_store_size(&store);
        assert_eq!(size, 0);

        // Inner capacity check. This should not be compacted.
        for i in 0..3 {
            store.set(&vec![0, 0, 0, i + 1], &value).unwrap();
            assert_eq!(get_store_size(&store), (i + 1) as usize);
            assert_eq!(store.get::<Vec<Vec<u8>>>(&vec![0, 0, 0, i + 1]).unwrap(), value);
        }

        // Inner capacity check. This should be not compacted.
        thread::sleep(Duration::from_secs(3));
        for i in 3..5 {
            store.set(&vec![0, 0, 0, i + 1], &value).unwrap();
            assert_eq!(get_store_size(&store), (i + 1) as usize);
            assert_eq!(store.get::<Vec<Vec<u8>>>(&vec![0, 0, 0, i + 1]).unwrap(), value);
        }

        // Store one more value. The store should be compacted.
        store.set(&vec![0, 0, 0, 4], &value).unwrap();
        assert_eq!(store.get::<Vec<Vec<u8>>>(&vec![0, 0, 0, 4]).unwrap(), value);
        assert_eq!(get_store_size(&store), 2); // compacted to 2.
        assert!(store.get::<Vec<Vec<u8>>>(&vec![0, 0, 0, 1]).is_none()); // removed by compaction.
        assert!(store.get::<Vec<Vec<u8>>>(&vec![0, 0, 0, 2]).is_none()); // removed by compaction.
        assert!(store.get::<Vec<Vec<u8>>>(&vec![0, 0, 0, 3]).is_none()); // removed by compaction.

        drop(store);
        fs::remove_dir_all(store_dir).unwrap();
    }
}
