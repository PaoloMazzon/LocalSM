use std::io::{BufRead, Seek};
use serde::{Deserialize, Serialize};
use crate::ilog::ilog::ILog;
use crate::ilog::util::SparseKeyIdTable;
use crate::util::bloomfilter::BloomFilter;
use crate::util::error::{LsmError, Result};

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
    reader: R,
}

impl<R: BufRead + Seek> SsTableLoader<R> {
    /// Loads the header out of an SsTable
    pub fn load(mut buffer: R) -> Result<Self> {
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
            reader: buffer,
        })
    }

    /// Returns true if a particular key exists on this table
    pub fn has(&self, key: &str, sort_key: i64) -> bool {
        self.get(key, sort_key).is_some()
    }

    /// Tries to get a key from an SSTable. Returns None if the requested key is not in the table.
    pub fn get(&self, key: &str, sort_key: i64) -> Option<&ILog> {
        if !self.bloom_filter.maybe_in_filter(key) {
            return None
        }
        let start_location = self.sparse_key_id_table.start_search_from_descending(key, Some(sort_key))?;

        todo!()
    }

    /// Collects all entries with a given key from a map
    pub fn get_all(&self, key: &str) -> Vec<&ILog> {
        todo!()
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
            maximum_records_per_compression_block: 5,
            ..Default::default()
        };
        mt.delete("key", 0).unwrap(); // 4
        mt.add("asd", 1, vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("fgh", 2, vec![0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("hjk", 3, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("qwe", 4, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("wer", 5, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap(); // 3
        mt.add("ert", 6, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("rty", 7, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("tyu", 8, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("yui", 9, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("uio", 10, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap(); // 2
        mt.add("iop", 11, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("op[", 12, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("p[]", 13, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("zxc", 14, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("xcv", 15, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap(); // 1
        mt.add("cvb", 16, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("vbn", 17, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("bnm", 18, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        mt.add("nm,", 19, vec![1, 1, 2, 3, 4, 5, 6, 7]).unwrap(); // 0
        mt.export_to_sstable(&config, &mut File::create(filename.as_str()).unwrap()).unwrap();
        filename
    }

    #[test]
    fn test_loading_from_memtable() {
        let filename = create_sstable();
        let file = BufReader::new(File::open(Path::new(filename.as_str())).unwrap());
        let loader = SsTableLoader::load(file).unwrap();
        assert_eq!(loader.bloom_filter.maybe_in_filter("nm,"), true, "nm, was not found in the bloom filter! File parsing probably went wrong.");
        assert_eq!(loader.bloom_filter.maybe_in_filter("cvb"), true, "cvb was not found in the bloom filter! File parsing probably went wrong.");
        assert_eq!(loader.bloom_filter.maybe_in_filter("vbn"), true, "vbn was not found in the bloom filter! File parsing probably went wrong.");
        assert_eq!(loader.bloom_filter.maybe_in_filter("bnm"), true, "bnm was not found in the bloom filter! File parsing probably went wrong.");
        assert_eq!(loader.bloom_filter.maybe_in_filter("asd"), true, "asd was not found in the bloom filter! File parsing probably went wrong.");
        assert_eq!(loader.bloom_filter.maybe_in_filter("fgh"), true, "fgh was not found in the bloom filter! File parsing probably went wrong.");
        assert_eq!(loader.bloom_filter.maybe_in_filter("hjk"), true, "hjk was not found in the bloom filter! File parsing probably went wrong.");
        assert_eq!(loader.sparse_key_id_table.len(), 5, "Sparse table was not serialized/deserialzed properly.");
    }
}
