use std::fs::File;
use serde::{Deserialize, Serialize};
use crate::ilog::ilog::ILog;
use crate::util::bloomfilter::BloomFilter;
use crate::util::error::LsmError;

/// Very simple header than can be parsed element-by-element in binary. The general model is the
/// parser will load this header and the bloom filter, then if the bloom filter passes, it will
/// find the exact record through the record table
#[derive(Serialize, Deserialize)]
struct BinaryILogHeader {
    /// Version of the binary encoding
    version: u32,
    
    /// Record table for fast lookups
    record_table: BinaryRecordTable,
    
    /// Bloom filter for this 
    bloom_filter: BloomFilter,
}

#[derive(Deserialize, Serialize)]
struct BinaryRecordTable {
    /// Offset from the start of the stream
    record_offset: u64,
    
    /// Name of this record
    record_name: String,
}

/// A record that is meant to be binary encoded
#[derive(Deserialize, Serialize)]
struct BinaryRecord {
    record: String,
}

pub fn ilog_to_file(ilog: ILog, file: File) -> Result<(), LsmError> {
    todo!("This");
}

pub fn find_in_ilog(key: String, file: File) -> Result<String, LsmError> {
    todo!("This");
}

pub fn parse_in_ilog(file: File) -> Result<ILog, LsmError> {
    todo!("This");
}
