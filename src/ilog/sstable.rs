use std::io::{BufRead};
use serde::{Deserialize, Serialize};
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
/*    ii. ILog struct of size previously stated                  */
/*****************************************************************/

/// Just an entry's key/id/location in file for SparseKeyIdTable
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct EntryLocation {
    /// Key for this record
    pub key: String,

    /// Sort key for this record
    pub sort_key: i64,

    /// How many bytes into the file from the top of the header this record is
    pub location: u64,
}

/// Contains a (sorted) list of key/id pairs and their locations in the file.
/// This does not contain all the keys, but can be used to get to right
/// ballpark quickly.
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct SparseKeyIdTable {
    pub key_ids: Vec<EntryLocation>,
}

/// Helps you navigate SSTables more easily
pub(crate) struct SsTableLoader<R: BufRead> {
    pub bloom_filter: BloomFilter,
    pub sparse_key_id_table: SparseKeyIdTable,
    reader: R,
}

impl<R: BufRead> SsTableLoader<R> {
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
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::BufReader;
    use std::path::Path;
    use crate::ilog::ilog::MemTable;
    use crate::ilog::sstable::SsTableLoader;
    use crate::util::config::Config;

    fn create_sstable() -> String {
        let filename = String::from("/tmp/test_sstable");
        let mut mt = MemTable::new("/tmp/asdkjakdjak.log".to_string()).unwrap();
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
    }
}
