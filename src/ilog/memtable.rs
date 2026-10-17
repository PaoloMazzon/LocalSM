use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use spdlog::error;
use crate::ilog::ilog::ILog;
use crate::ilog::sstable::{SSTABLE_ENCODING_VERSION, SSTABLE_MAGIC_BYTES};
use crate::ilog::util::{CompressionBlock, EntryLocation, SparseKeyIdTable};
use crate::util::bloomfilter::BloomFilter;
use crate::util::config::Config;
use crate::util::time::get_iso_time;

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
    pub fn new(cache_file: String) -> crate::util::error::Result<Self> {
        Ok(Self {
            records: Vec::new(),
            cache_file: File::create(Path::new(cache_file.as_str()))?,
            cache_file_location: cache_file,
        })
    }

    /// Creates a MemTable by recovering from a cache file
    pub fn recover(cache_file_location: &str) -> crate::util::error::Result<Self> {
        let file = File::open(cache_file_location)?;
        let reader = BufReader::new(file);

        let cache_file = OpenOptions::new()
            .append(true)
            .open(cache_file_location)?;
        let mut return_table = Self {
            records: Vec::new(),
            cache_file,
            cache_file_location: cache_file_location.to_string(),
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
    fn add_to_log(&mut self, line: String) -> crate::util::error::Result<()> {
        let total_string = line + "\n";
        self.cache_file.write_all(total_string.as_bytes())?;
        self.cache_file.flush()?;
        Ok(())
    }

    /// Adds a new record to the in-memory map and the crash-recovery file.
    /// This does not assign unique sort_keys as those must be acquired elsewhere.
    pub fn add(&mut self, key: &str, sort_key: i64, val: Vec<u8>) -> crate::util::error::Result<()> {
        let log = ILog {
            key: key.to_string(),
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
    pub fn delete(&mut self, key: &str, sort_key: i64) -> crate::util::error::Result<()> {
        let log = ILog {
            key: key.to_string(),
            sort_key,
            value: vec![],
            timestamp: get_iso_time(),
            tombstone: true,
        };
        self.add_to_log(serde_json::to_string(&log)?)?;
        self.records.push(log);
        Ok(())
    }

    /// Returns true if a key exists in this MemTable (and isn't a tombstone)
    pub fn has(&self, key: &str, sort_key: i64) -> bool {
        match self.records.iter().rfind(|log| log.key == key && log.sort_key == sort_key) {
            Some(x) => !x.tombstone,
            None => false
        }
    }

    /// Returns a requested log from a MemTable or None if it isn't present
    pub fn get(&self, key: &str, sort_key: i64) -> Option<&ILog> {
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

    /// Creates a sparse table and all the compressed blocks at the same time, since the sparse
    /// table needs exact sizes. This is a very heavy function is the majority of the CPU time
    /// involved in generating an SSTable.
    fn create_sparse_table(&mut self, config: &Config) -> crate::util::error::Result<(SparseKeyIdTable, Vec<Vec<u8>>)> {
        self.records.sort_unstable();
        self.records.reverse();

        let mut table = SparseKeyIdTable {
            key_ids: Vec::new()
        };

        // We write records every n times, counting the distance to each
        let mut total_size = 0;
        let mut compression_blocks = vec![];
        let mut current_compression_block= CompressionBlock::new();

        for record in &self.records {
            // If its a new compression block, add this element to the sparse table
            if current_compression_block.logs.len() == 0 {
                table.key_ids.push(
                    EntryLocation {
                        key: record.key.to_string(),
                        sort_key: record.sort_key,
                        location: total_size,
                    }
                )
            }

            // Add the current entry to the current compression block
            current_compression_block.add_to_block(record.clone());

            // If we should close up this compression block, record the size and move the size
            if current_compression_block.size() >= config.size_of_compression_blocks as u64 || current_compression_block.logs.len() >= config.maximum_records_per_compression_block as usize {
                let compressed = current_compression_block.compress()?;
                total_size += 4 + compressed.len() as u64;
                compression_blocks.push(compressed);
                current_compression_block = CompressionBlock::new();
            }
        }

        // Wrap up the last compression block
        if current_compression_block.logs.len() != 0 {
            let compressed = current_compression_block.compress()?;
            total_size += 4 + compressed.len() as u64;
            compression_blocks.push(compressed);
        }

        // We record the last entry to the sparse table as well so we can quickly tell if something
        // is out of range of the SSTable at a glance
        if let Some(record) = self.records.last() {
            table.key_ids.push(
                EntryLocation {
                    key: record.key.to_string(),
                    sort_key: record.sort_key,
                    location: total_size,
                }
            );
        }

        Ok((table, compression_blocks))
    }

    /// Exports the whole in-memory table to an immutable SSTable that can be dumped
    /// to a file or NAS or whatever. This can write partial amounts then fail.
    pub fn export_to_sstable(&mut self, config: &Config, dest: &mut impl std::io::Write) -> crate::util::error::Result<()> {
        // Encode the header information, including the bloom filter and sparse table
        dest.write_all(&SSTABLE_MAGIC_BYTES)?;
        dest.write_all(SSTABLE_ENCODING_VERSION.to_le_bytes().as_slice())?;
        let bloom_filter = postcard::to_allocvec(&self.create_bloom_filter(config))?;
        let (sparse_table, compressed_blocks) = self.create_sparse_table(config)?;
        let sparse_table_serialized = postcard::to_allocvec(&sparse_table)?;
        dest.write_all((bloom_filter.len() as u32).to_le_bytes().as_slice())?;
        dest.write_all(&bloom_filter)?;
        dest.write_all((sparse_table_serialized.len() as u32).to_le_bytes().as_slice())?;
        dest.write_all(&sparse_table_serialized)?;

        // Iterate over all compressed blocks and dump them to file
        for compressed_block in &compressed_blocks {
            dest.write_all((compressed_block.len() as u32).to_le_bytes().as_slice())?;
            dest.write_all(&compressed_block)?;
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
        assert!(!mt.has("key", 10), "\"has\" is detecting a non-existant record.");
        mt.add("key", 10, vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        assert!(mt.has("key", 10), "Adding or \"has\" isn't working.");
        mt.delete("key", 10).unwrap();
        assert!(!mt.has("key", 10), "Deleting records didn't work.");
    }

    #[test]
    fn test_add_many() {
        let mut mt = MemTable::new("/tmp/test.log2".to_string()).unwrap();
        mt.add("key", 10, vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("key", 11, vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("key", 12, vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("key2", 10, vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("key2", 11, vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("key2", 12, vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("key3", 100, vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
    }

    #[test]
    fn test_recovery() {
        let mut mt = MemTable::new("/tmp/recover.log".to_string()).unwrap();
        mt.add("key", 10, vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.delete("key", 10).unwrap();
        mt.add("key", 11, vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("key", 12, vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("test", 20, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();

        let loaded_table = MemTable::recover("/tmp/recover.log").unwrap();
        assert!(!loaded_table.has("key", 10), "Tombstone was not loaded properly.");
        assert!(loaded_table.has("key", 11), "Value was not loaded properly.");
        assert!(loaded_table.has("key", 12), "Value was not loaded properly.");
        assert_eq!(loaded_table.get("test", 20).unwrap().value, vec![1, 1, 2, 3, 4, 5, 6, 7], "Value was not loaded or parsed properly.");
    }

    #[test]
    fn test_serializing_sstable() {
        let mut mt = MemTable::new("/tmp/fake_recover.log".to_string()).unwrap();
        let config = Config {
            maximum_records_per_compression_block: 5,
            ..Default::default()
        };
        mt.delete("key", 0).unwrap(); // sstable entry
        mt.add("asd", 1, vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("fgh", 2, vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("hjk", 3, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("qwe", 4, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("wer", 5, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap(); // sstable entry
        mt.add("ert", 6, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("rty", 7, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("tyu", 8, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("yui", 9, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("uio", 10, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap(); // sstable entry
        mt.add("iop", 11, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("op[", 12, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("p[]", 13, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("zxc", 14, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("xcv", 15, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap(); // sstable entry
        mt.add("cvb", 16, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("vbn", 17, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("bnm", 18, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("nm,", 19, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap(); // sstable entry
        mt.export_to_sstable(&config, &mut File::create("/tmp/test_sstable.log").unwrap()).unwrap();
    }

    #[test]
    fn test_sparse_lookup() {
        let mut mt = MemTable::new("/tmp/asdkjakdjak.log".to_string()).unwrap();
        let config = Config {
            maximum_records_per_compression_block: 4,
            ..Default::default()
        };
        mt.add("1", 1, vec![0, 1]).unwrap(); // second/third entry
        mt.add("2", 8, vec![0, 1]).unwrap();
        mt.add("3", 1, vec![0, 1]).unwrap();
        mt.add("4", 1, vec![0, 1]).unwrap();
        mt.add("4", 3, vec![0, 1]).unwrap(); // first entry
        mt.add("6", 1, vec![0, 1]).unwrap();
        mt.add("6", 1, vec![2, 3]).unwrap();
        mt.add("8", 9, vec![0, 1]).unwrap();
        mt.add("9", 1, vec![0, 1]).unwrap(); // zeroth entry
        let (sparse_table, _) = mt.create_sparse_table(&config).unwrap();
        println!("{:#?}", sparse_table);
        assert_eq!(sparse_table.len(), 4, "Sparse table was generated wrong.");
        assert_eq!(sparse_table.start_search_from_descending("1").unwrap(), sparse_table.key_ids[1].location, "Sparse table key was in the wrong location.");
        assert_eq!(sparse_table.start_search_from_descending("2").unwrap(), sparse_table.key_ids[1].location, "Sparse table key was in the wrong location.");
        assert_eq!(sparse_table.start_search_from_descending("3").unwrap(), sparse_table.key_ids[1].location, "Sparse table key was in the wrong location.");
        assert_eq!(sparse_table.start_search_from_descending("4").unwrap(), sparse_table.key_ids[0].location, "Sparse table key was in the wrong location.");
        assert_eq!(sparse_table.start_search_from_descending("6").unwrap(), sparse_table.key_ids[0].location, "Sparse table key was in the wrong location.");
        assert_eq!(sparse_table.start_search_from_descending("8").unwrap(), sparse_table.key_ids[0].location, "Sparse table key was in the wrong location.");
        assert_eq!(sparse_table.start_search_from_descending("9").unwrap(), sparse_table.key_ids[0].location, "Sparse table key was in the wrong location.");
    }
}
