use std::io::BufRead;
use serde::{Deserialize, Serialize};
use crate::util::bloomfilter::BloomFilter;
use crate::util::error::Result;

pub(crate) static SSTABLE_MAGIC_BYTES: [u8; 4] = [255, 92, 54, 92];
pub(crate) static SSTABLE_ENCODING_VERSION: u32 = 1;

/******************** SSTable binary format v1 *******************/
/* 1. 4 byte header                                              */
/* 1. 4 version                                                  */
/* 2. 4 bytes for the size of BloomFilter struct                 */
/* 3. BloomFilter struct of size previously stated in (2)        */
/* 4. Size of SparseKeyIdTable                                   */
/* 5. SparseKeyIdTable struct of size previously stated in (4)   */
/* 6. For the remainder of the file (until EOF),                 */
/*     i. Size of the entry at this location (its a struct ILog) */
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

/// Returns true if a table contains a given key/id pair
pub(crate) fn table_contains(buffer: impl BufRead, id: i64, key: String) -> Result<bool> {
    todo!("")
}

/// Returns true if a table contains a key with any id
pub(crate) fn table_contains_key(buffer: impl BufRead, key: String) -> Result<bool> {
    todo!("")
}

/// Returns the bloom filters in an sstable
pub(crate) fn get_bloom_filter(buffer: impl BufRead) -> Result<BloomFilter> {
    todo!("")
}

/// Gets a value from an sstable, can fail if that value doesn't exist in it
pub(crate) fn get_from_table(buffer: impl BufRead, id: i64, key: String) -> Result<Vec<u8>> {
    todo!("")
}
