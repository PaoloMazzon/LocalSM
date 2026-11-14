use std::io::{BufRead, Seek, SeekFrom};
use spdlog::{error, warn, Logger};
use crate::ilog::ilog::ILog;
use crate::ilog::util::{CompressionBlock, SparseKeyIdTable};
use crate::util::bloomfilter::BloomFilter;
use crate::util::config::Config;
use crate::util::error::{LsmError, Result};
use crate::util::error::LsmError::EntryNotPresent;

pub(crate) static SSTABLE_MAGIC_BYTES: [u8; 4] = [255, 92, 54, 92];
pub(crate) static SSTABLE_ENCODING_VERSION: u32 = 1;

/******************** SSTable binary format v1 *******************/
/* 1. 4 byte header                                              */
/* 1. 4 version                                                  */
/* 2. 4 bytes for the size of BloomFilter struct (u32)           */
/* 3. BloomFilter struct of size previously stated in (2)        */
/* 4. Size of SparseKeyIdTable (u32)                             */
/* 5. SparseKeyIdTable struct of size previously stated in (4)   */
/* 6. For the remainder of the file (until EOF),                 */
/*     i. Size of the entry at this location (u32)               */
/*    ii. CompressionBlock of size previously state              */
/* Note: Sparse table entries point to the first element in the  */
/* CompressionBlock. This means for targeted lookups you will    */
/* only ever need to decompress at most 1 block.                 */
/*****************************************************************/

/// Helps you navigate SSTables more easily
pub(crate) struct SsTableLoader<R: BufRead + Seek> {
    pub bloom_filter: BloomFilter,
    pub sparse_key_id_table: SparseKeyIdTable,
    end_of_header: u64,
    reader: R,
    config: Config,
    logger: Logger,
}

impl<R: BufRead + Seek> SsTableLoader<R> {
    /// Gets a compression block from the reader, assuming the pointer is currently at the start
    /// of the size of the compression block.
    fn get_compression_block(&mut self) -> Result<CompressionBlock> {
        let size_of_block = self.reader.read_le::<u32>()?;
        let mut bytes = Vec::new();
        bytes.resize(size_of_block as usize, 0);
        self.reader.read_exact(bytes.as_mut_slice())?;
        let b = CompressionBlock::decompress(bytes.as_slice())?;
        Ok(b)
    }

    /// Loads the header out of an SsTable
    pub fn load(mut buffer: R, logger: Logger, config: Config) -> Result<Self> {
        // Validate magic bytes
        {
            let mut magic_bytes: [u8; 4] = [0, 0, 0, 0];
            buffer.read_exact(&mut magic_bytes)?;
            if SSTABLE_MAGIC_BYTES != magic_bytes {
                return Err(LsmError::InvalidStringTable(format!("Magic bytes for SSTable were invalid (found {:?}, expected {:?})", magic_bytes, SSTABLE_MAGIC_BYTES)))
            }
        }

        // Validate encoding version
        let encoding_version = buffer.read_le::<u32>()?;
        if encoding_version != SSTABLE_ENCODING_VERSION {
            return Err(LsmError::InvalidStringTable(format!("Found SSTable with incompatible encoding version {}", encoding_version)))
        }

        // Start pulling out various pieces of the header
        let bloom_filter: BloomFilter = {
            let size = buffer.read_le::<u32>()?;
            let mut bloom_bytes = Vec::new();
            bloom_bytes.resize(size as usize, 0);
            buffer.read_exact(bloom_bytes.as_mut_slice())?;
            postcard::from_bytes(bloom_bytes.as_slice())?
        };
        let sparse_key_id_table: SparseKeyIdTable = {
            let size = buffer.read_le::<u32>()?;
            let mut sparse_bytes = Vec::new();
            sparse_bytes.resize(size as usize, 0);
            buffer.read_exact(sparse_bytes.as_mut_slice())?;
            postcard::from_bytes(sparse_bytes.as_slice())?
        };

        Ok(Self {
            bloom_filter,
            sparse_key_id_table,
            end_of_header: buffer.stream_position()?,
            reader: buffer,
            config,
            logger,
        })
    }

    /// Returns true if a particular key exists on this table. None will be returned if the key
    /// cannot be found for any reason, including but not limited to:
    ///  1. The block it's in being corrupted
    ///  2. The file or stream being corrupted
    ///  3. The file ends prematurely
    /// Use `get` for more accurate results.
    pub fn has(&mut self, key: &str, sort_key: i64) -> bool {
        self.get(key, sort_key).is_ok()
    }

    /// Tries to get a key from an SSTable. Returns None if the requested key is not in the table.
    /// # Note
    /// This does *not* consider tombstones.
    pub fn get(&mut self, key: &str, sort_key: i64) -> Result<ILog> {
        if !self.bloom_filter.maybe_in_filter(key) {
            return Err(LsmError::EntryNotPresent)
        }
        let start_location = self.sparse_key_id_table.start_search_from_descending(key, Some(sort_key))
            .ok_or_else(|| LsmError::EntryNotPresent)?;
        self.reader.seek(SeekFrom::Start(self.end_of_header + start_location))?;

        // We loop because it's possible for an entire block to be the same sort key and primary key
        // if they all have different timestamps.
        loop {
            let block = self.get_compression_block()?;
            for log in &block.logs {
                if log.key.as_str() < key || (log.key.as_str() == key && log.sort_key > sort_key) {
                    return Err(EntryNotPresent)
                }
                if log.key.as_str() == key && log.sort_key == sort_key {
                    return Ok(log.clone())
                }
            }
        }
    }

    /// Collects all entries with a given key from a map, this function will clobber most stream
    /// related errors implicitly. Prefer get where possible.
    pub fn get_all(&mut self, key: &str) -> Result<Vec<ILog>> {
        let mut out_vec = Vec::new();
        if !self.bloom_filter.maybe_in_filter(key) {
            return Ok(out_vec)
        }
        let start_location = match self.sparse_key_id_table.start_search_from_descending(key, None) {
            Some(s) => s,
            None => return Ok(out_vec)
        };
        self.reader.seek(SeekFrom::Start(self.end_of_header + start_location))?;

        loop {
            let block = match self.get_compression_block() {
                Ok(b) => b,
                Err(_) => return Ok(out_vec)
            };
            for log in &block.logs {
                if log.key.as_str() < key {
                    return Ok(out_vec)
                }
                if log.key.as_str() == key {
                    out_vec.push(log.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::BufReader;
    use std::path::Path;
    use crate::ilog::memtable;
    use crate::ilog::memtable::MemTable;
    use crate::ilog::sstable::{SparseKeyIdTable, SsTableLoader};
    use crate::ilog::util::EntryLocation;
    use crate::util::config::Config;

    fn create_sstable() -> String {
        let filename = String::from("/tmp/test_sstable");
        let mut mt = MemTable::new("/tmp/asdkjakdjak.log".to_string()).unwrap();
        let config = Config {
            sstable_config: crate::util::config::SsTableConfig {
                maximum_records_per_compression_block: 5,
                ..Default::default()
            },
            ..Default::default()
        };
        mt.delete("key", 0).unwrap(); // 4
        mt.add("asd", 1, vec![0, 2]).unwrap();
        mt.add("fgh", 2, vec![0, 2]).unwrap();
        mt.add("hjk", 3, vec![1, 2]).unwrap();
        mt.add("qwe", 4, vec![1, 2]).unwrap();
        mt.add("wer", 5, vec![1, 2]).unwrap(); // 3
        mt.add("ert", 6, vec![1, 2]).unwrap();
        mt.add("rty", 7, vec![1, 2]).unwrap();
        mt.add("tyu", 8, vec![2, 3]).unwrap();
        mt.add("yui", 9, vec![1, 2]).unwrap();
        mt.add("uio", 10, vec![1, 2]).unwrap(); // 2
        mt.add("iop", 11, vec![1, 2]).unwrap();
        mt.add("op[", 12, vec![1, 2]).unwrap();
        mt.add("p[]", 13, vec![1, 2]).unwrap();
        mt.add("zxc", 14, vec![1, 2]).unwrap();
        mt.add("xcv", 15, vec![1, 2]).unwrap(); // 1
        mt.add("cvb", 16, vec![1, 2]).unwrap();
        mt.add("vbn", 17, vec![1, 2]).unwrap();
        mt.add("bnm", 18, vec![1, 2]).unwrap();
        mt.add("bnm", 17, vec![3, 4]).unwrap();
        mt.add("bnm", 16, vec![5, 6]).unwrap();
        mt.add("bnm", 15, vec![6, 7]).unwrap();
        mt.add("nm,", 19, vec![1, 2]).unwrap(); // 0
        mt.export_to_sstable(&config, &mut File::create(filename.as_str()).unwrap()).unwrap();
        filename
    }

    #[test]
    fn test_loading_from_sstable() {
        let filename = create_sstable();
        let file = BufReader::new(File::open(Path::new(filename.as_str())).unwrap());
        let mut loader = SsTableLoader::load(file, spdlog::Logger::builder().build().unwrap(), Config::default()).unwrap();

        // Validate bloom filter
        assert_eq!(loader.bloom_filter.maybe_in_filter("nm,"), true, "nm, was not found in the bloom filter! File parsing probably went wrong.");
        assert_eq!(loader.bloom_filter.maybe_in_filter("cvb"), true, "cvb was not found in the bloom filter! File parsing probably went wrong.");
        assert_eq!(loader.bloom_filter.maybe_in_filter("vbn"), true, "vbn was not found in the bloom filter! File parsing probably went wrong.");
        assert_eq!(loader.bloom_filter.maybe_in_filter("bnm"), true, "bnm was not found in the bloom filter! File parsing probably went wrong.");
        assert_eq!(loader.bloom_filter.maybe_in_filter("asd"), true, "asd was not found in the bloom filter! File parsing probably went wrong.");
        assert_eq!(loader.bloom_filter.maybe_in_filter("fgh"), true, "fgh was not found in the bloom filter! File parsing probably went wrong.");
        assert_eq!(loader.bloom_filter.maybe_in_filter("hjk"), true, "hjk was not found in the bloom filter! File parsing probably went wrong.");
        assert_eq!(loader.sparse_key_id_table.len(), 6, "Sparse table was not serialized/deserialzed properly.");

        // Validate get function
        assert_eq!(loader.get("tyu", 8).unwrap().value, vec![2, 3], "Failed to find a value in an SSTable");
        assert_eq!(loader.get_all("tyu").unwrap().len(), 1, "Get all not functioning properly");
        assert_eq!(loader.get_all("bnm").unwrap().len(), 4, "Get all not functioning properly");
    }
}
