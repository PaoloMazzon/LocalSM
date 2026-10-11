use std::cmp::Ordering;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use serde::{Deserialize, Serialize};
use spdlog::prelude::*;
use crate::ilog::sstable::{EntryLocation, SparseKeyIdTable, SSTABLE_ENCODING_VERSION, SSTABLE_MAGIC_BYTES};
use crate::util::bloomfilter::BloomFilter;
use crate::util::config::Config;
use crate::util::error::Result;
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
    cache_file_location: String,
}

impl MemTable {
    /// Creates a new MemTable that uses a specified cache file as its crash recovery
    /// file. It does not attempt to load from that file, it will be overwritten.
    pub fn new(cache_file: String) -> Result<Self> {
        Ok(Self {
            records: Vec::new(),
            cache_file: File::create(Path::new(cache_file.as_str()))?,
            cache_file_location: cache_file,
        })
    }

    /// Creates a MemTable by recovering from a cache file
    pub fn recover(cache_file_location: String) -> Result<Self> {
        let file = File::open(cache_file_location.as_str())?;
        let reader = BufReader::new(file);

        let cache_file = OpenOptions::new()
            .append(true)
            .open(cache_file_location.as_str())?;
        let mut return_table = Self {
            records: Vec::new(),
            cache_file,
            cache_file_location: cache_file_location.clone(),
        };

        // Iterate over the lines lazily
        for line in reader.lines() {
            match line {
                Ok(json) => {
                    return_table.records.push(serde_json::from_str(json.as_str())?);
                },
                Err(e) => {
                    error!("Failed to recover line from file {}, {:?}", cache_file_location, e);
                    break;
                }
            }
        }

        Ok(return_table)
    }

    /// Adds a line to the log file with a trailing newline
    fn add_to_log(&mut self, line: String) -> Result<()> {
        let total_string = line + "\n";
        self.cache_file.write_all(total_string.as_bytes())?;
        self.cache_file.flush()?;
        Ok(())
    }

    /// Adds a new record to the in-memory map and the crash-recovery file.
    /// This does not assign unique sort_keys as those must be acquired elsewhere.
    pub fn add(&mut self, sort_key: i64, key: String, val: Vec<u8>) -> Result<()> {
        let log = ILog {
            key: key.clone(),
            sort_key,
            value: val,
            timestamp: get_iso_time(),
            tombstone: false,
        };
        self.add_to_log(serde_json::to_string(&log)?)?;
        self.records.push(log);
        Ok(())
    }

    /// Deletes a record
    pub fn delete(&mut self, sort_key: i64, key: String) -> Result<()> {
        let log = ILog {
            key: key.clone(),
            sort_key,
            value: vec![],
            timestamp: get_iso_time(),
            tombstone: true,
        };
        self.add_to_log(serde_json::to_string(&log)?)?;
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

    /// Creates a bloom filter from the existing records in this MemTable
    fn create_bloom_filter(&self, config: &Config) -> BloomFilter {
        let mut bloom = BloomFilter::init(config.bloom_filter_bits as usize, config.bloom_filter_hashes as usize);
        for record in &self.records {
            bloom.add(record.key.as_str());
        }
        bloom
    }

    /// Creates the sparse key/id table for this MemTable and returns it. This is quite heavy
    /// as it needs to sort the entire records list and binary encode every member to get its
    /// size for the entries sparse lookup table.
    fn create_sparse_table(&mut self, config: &Config) -> Result<SparseKeyIdTable> {
        self.records.sort_unstable_by(|x, y| {
            if x.key < y.key {
                return Ordering::Less
            } else if x.key > y.key {
                return Ordering::Greater
            }
            if x.sort_key < y.sort_key {
                return Ordering::Less
            } else if x.sort_key > y.sort_key {
                return Ordering::Greater
            }
            Ordering::Equal
        });

        let mut table = SparseKeyIdTable {
            key_ids: Vec::new()
        };

        // We write records every n times, counting the distance to each
        let mut total_size = 0;
        let mut counter = 0;
        for record in &self.records {
            // Record this entry
            if counter % config.sparse_table_record_count == 0 {
                table.key_ids.push(EntryLocation {
                    key: record.key.clone(),
                    sort_key: record.sort_key,
                    location: total_size,
                })
            }

            // Just add the size of this record
            total_size += 4;
            total_size += postcard::to_allocvec(record)?.len() as u64;
            counter += 1;
        }

        Ok(table)
    }

    /// Exports the whole in-memory table to an immutable SSTable that can be dumped
    /// to a file or NAS or whatever. This can write partial amounts then fail.
    pub fn export_to_sstable(&mut self, config: &Config, dest: &mut impl std::io::Write) -> Result<()> {
        // Encode the header information, including the bloom filter and sparse table
        dest.write_all(&SSTABLE_MAGIC_BYTES)?;
        dest.write_all(SSTABLE_ENCODING_VERSION.to_le_bytes().as_slice())?;
        let bloom_filter = postcard::to_allocvec(&self.create_bloom_filter(config))?;
        let sparse_table = postcard::to_allocvec(&self.create_sparse_table(config)?)?;
        dest.write_all((bloom_filter.len() as u32).to_le_bytes().as_slice())?;
        dest.write_all(&bloom_filter)?;
        dest.write_all((sparse_table.len() as u32).to_le_bytes().as_slice())?;
        dest.write_all(&sparse_table)?;

        // Iterate over all records and encode those too
        for record in &self.records {
            let binary_record = postcard::to_allocvec(record)?;
            dest.write_all((binary_record.len() as u32).to_le_bytes().as_slice())?;
            dest.write_all(&binary_record)?;
        }

        Ok(())
    }
}

impl Drop for MemTable {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_file(Path::new(self.cache_file_location.as_str())) {
            error!("Failed to remove cache file for a MemTable at {}, {:?}", self.cache_file_location, e);
        }
    }
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

    #[test]
    fn test_recovery() {
        let mut mt = MemTable::new("/tmp/recover.log".to_string()).unwrap();
        mt.add(10, "key".to_string(), vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.delete(10, "key".to_string()).unwrap();
        mt.add(11, "key".to_string(), vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(12, "key".to_string(), vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(20, "test".to_string(), vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();

        let loaded_table = MemTable::recover("/tmp/recover.log".to_string()).unwrap();
        assert!(!loaded_table.has(10, "key".to_string()), "Tombstone was not loaded properly.");
        assert!(loaded_table.has(11, "key".to_string()), "Value was not loaded properly.");
        assert!(loaded_table.has(12, "key".to_string()), "Value was not loaded properly.");
        assert_eq!(loaded_table.get(20, "test".to_string()).unwrap().value, vec![1, 1, 2, 3, 4, 5, 6, 7], "Value was not loaded or parsed properly.");
    }

    #[test]
    fn test_serializing_sstable() {
        let mut mt = MemTable::new("/tmp/fake_recover.log".to_string()).unwrap();
        let config = Config {
            sparse_table_record_count: 5,
            entries_per_sstable: 20,
            ..Default::default()
        };
        mt.delete(0, "key".to_string()).unwrap();
        mt.add(1, "asd".to_string(), vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(2, "fgh".to_string(), vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(3, "hjk".to_string(), vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(4, "qwe".to_string(), vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(5, "wer".to_string(), vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(6, "ert".to_string(), vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(7, "rty".to_string(), vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(8, "tyu".to_string(), vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(9, "yui".to_string(), vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(10, "uio".to_string(), vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(11, "iop".to_string(), vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(12, "op[".to_string(), vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(13, "p[]".to_string(), vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(14, "zxc".to_string(), vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(15, "xcv".to_string(), vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(16, "cvb".to_string(), vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(17, "vbn".to_string(), vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(18, "bnm".to_string(), vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add(19, "nm,".to_string(), vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.export_to_sstable(&config, &mut File::create("/tmp/test_sstable.log").unwrap()).unwrap();
    }
}
