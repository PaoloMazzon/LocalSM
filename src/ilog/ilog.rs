use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::util::error::LsmError;
use crate::util::time::get_iso_time;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ILog {
    /// String key for this one, only necessary for crash log as its also stored in the btreemap
    key: String,

    /// 64-bit integer representing the immutable id of this record
    id: i64,

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
    records: BTreeMap<String, ILog>,
    cache_file: File,
}

impl MemTable {
    /// Creates a new MemTable that uses a specified cache file as its crash recovery
    /// file. It does not attempt to load from that file, it will be overwritten.
    pub fn new(cache_file: String) -> Result<Self, LsmError> {
        Ok(Self {
            records: BTreeMap::new(),
            cache_file: File::open(Path::new(cache_file.as_str())).map_err(|e| LsmError::FileNotAvailable(format!("{:?}", e)))?,
        })
    }

    /// Creates a MemTable by recovering from a cache file
    pub fn recover(cache_file: String) -> Self {
        todo!("this")
    }

    /// Adds a new record to the in-memory map and the crash-recovery file.
    /// This does not assign unique IDs as those must be acquired elsewhere.
    pub fn add(&mut self, id: i64, key: String, val: Vec<u8>) -> Result<(), LsmError> {
        let log = ILog {
            key: key.clone(),
            id,
            value: val,
            timestamp: get_iso_time(),
            tombstone: false,
        };
        self.cache_file.write_all(serde_json::to_string(&log)
            .map_err(|e| LsmError::JsonEncodingError(format!("{:?}", e)))?.as_bytes())
            .map_err(|e| LsmError::FileNotAvailable(format!("{:?}", e)))?;
        self.records.insert(key, log);
        Ok(())
    }

    /// Deletes a record
    pub fn delete(&mut self, id: i64, key: String) -> Result<(), LsmError> {
        let log = ILog {
            key: key.clone(),
            id,
            value: vec![],
            timestamp: get_iso_time(),
            tombstone: true,
        };
        self.cache_file.write_all(serde_json::to_string(&log)
            .map_err(|e| LsmError::JsonEncodingError(format!("{:?}", e)))?.as_bytes())
            .map_err(|e| LsmError::FileNotAvailable(format!("{:?}", e)))?;
        self.records.insert(key, log);
        Ok(())
    }

    // TODO: Serialize to an SSTable
}