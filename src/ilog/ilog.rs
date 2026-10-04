use std::fs::File;
use std::io::Write;
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::util::error::LsmError;
use crate::util::time::get_iso_time;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ILog {
    /// String key for this one
    key: String,

    /// 64-bit integer that can be used to sort records
    sort_key: i64,

    /// Payload in raw binary
    value: Vec<u8>,

    /// ISO-8601 formatted string
    timestamp: String,

    /// If this is a deletion record
    tombstone: bool,
}

/// In-memory i-logs
#[derive(Debug)]
pub(crate) struct MemTable {
    records: Vec<ILog>,
    cache_file: File,
}

impl MemTable {
    /// Creates a new MemTable that uses a specified cache file as its crash recovery
    /// file. It does not attempt to load from that file, it will be overwritten.
    pub fn new(cache_file: String) -> Result<Self, LsmError> {
        Ok(Self {
            records: Vec::new(),
            cache_file: File::create(Path::new(cache_file.as_str())).map_err(|e| LsmError::FileNotAvailable(format!("{:?}", e)))?,
        })
    }

    /// Creates a MemTable by recovering from a cache file
    pub fn recover(cache_file: String) -> Self {
        todo!("this")
    }

    /// Adds a new record to the in-memory map and the crash-recovery file.
    /// This does not assign unique sort_keys as those must be acquired elsewhere.
    pub fn add(&mut self, sort_key: i64, key: String, val: Vec<u8>) -> Result<(), LsmError> {
        let log = ILog {
            key: key.clone(),
            sort_key,
            value: val,
            timestamp: get_iso_time(),
            tombstone: false,
        };
        self.cache_file.write_all(serde_json::to_string(&log)
            .map_err(|e| LsmError::JsonEncodingError(format!("{:?}", e)))?.as_bytes())
            .map_err(|e| LsmError::FileNotAvailable(format!("{:?}", e)))?;
        self.records.push(log);
        Ok(())
    }

    /// Deletes a record
    pub fn delete(&mut self, sort_key: i64, key: String) -> Result<(), LsmError> {
        let log = ILog {
            key: key.clone(),
            sort_key,
            value: vec![],
            timestamp: get_iso_time(),
            tombstone: true,
        };
        self.cache_file.write_all(serde_json::to_string(&log)
            .map_err(|e| LsmError::JsonEncodingError(format!("{:?}", e)))?.as_bytes())
            .map_err(|e| LsmError::FileNotAvailable(format!("{:?}", e)))?;
        self.records.push(log);
        Ok(())
    }

    pub fn has(&self, sort_key: i64, key: String) -> bool {
        match self.records.iter().rfind(|log| log.key == key && log.sort_key == sort_key) {
            Some(x) => !x.tombstone,
            None => false
        }
    }

    pub fn get(&self, sort_key: i64, key: String) -> Option<&ILog> {
        match self.records.iter().rfind(|log| log.key == key && log.sort_key == sort_key) {
            Some(x) => match x.tombstone {
                false => Some(x),
                true => None
            },
            None => None
        }
    }

    // TODO: Serialize to an SSTable
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_get_set_remove() {
        let mut mt = MemTable::new("/tmp/test.log".to_string()).unwrap();
        assert!(!mt.has(10, "key".to_string()), "\"has\" is detecting a non-existant record.");
        mt.add(10, "key".to_string(), vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        assert!(mt.has(10, "key".to_string()), "Adding or \"has\" isn't working.");
        mt.delete(10, "key".to_string()).unwrap();
        assert!(!mt.has(10, "key".to_string()), "Deleting records didn't work.");
    }

    #[test]
    fn test_add_many() {
        let mut mt = MemTable::new("/tmp/test.log2".to_string()).unwrap();
        mt.add(10, "key".to_string(), vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(11, "key".to_string(), vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(12, "key".to_string(), vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(10, "key2".to_string(), vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(11, "key2".to_string(), vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(12, "key2".to_string(), vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(100, "key3".to_string(), vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
    }
}
