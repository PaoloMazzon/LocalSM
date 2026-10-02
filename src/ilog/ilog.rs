use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ILog {
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
    cache_location: String,
}

impl MemTable {
    // TODO: Adding elements also pushes to the crash log
    // TODO: Serialize to an SSTable
}