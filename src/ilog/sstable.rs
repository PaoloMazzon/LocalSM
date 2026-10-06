use std::io::BufRead;
use serde::{Deserialize, Serialize};
use crate::util::bloomfilter::BloomFilter;
use crate::util::error::LsmError;

/********************* SSTable binary format *********************/
/* 1. 4 byte header                                              */
/* 2. 4 bytes for the size of BloomFilterPair struct             */
/* 3. BloomFilterPair struct of size previously stated in (2)    */
/* 4. Size of SparseKeyIdTable                                   */
/* 5. SparseKeyIdTable struct of size previously stated in (4)   */
/* 6. For the remainder of the file (until EOF),                 */
/*     i. Size of the entry at this location (its a struct ILog) */
/*    ii. ILog struct of size previously stated                  */
/*****************************************************************/

/// Just an entry's key/id/location in file for SparseKeyIdTable
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct EntryLocation {
    pub key: String,
    pub id: i64,
    pub location: u64,
}

/// Contains a (sorted) list of key/id pairs and their locations in the file.
/// This does not contain all the keys, but can be used to get to right
/// ballpark quickly.
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct SparseKeyIdTable {
    key_ids: Vec<EntryLocation>,
}

/// Each table contains a bloom filter for the ids and keys
#[derive(Serialize, Deserialize)]
pub(crate) struct BloomFilterPair {
    id_filter: BloomFilter,
    key_filter: BloomFilter,
}

/// Returns true if a table contains a given key/id pair
pub(crate) fn table_contains(buffer: impl BufRead, id: i64, key: String) -> Result<bool, LsmError> {
    todo!("")
}

/// Returns true if a table contains a key with any id
pub(crate) fn table_contains_key(buffer: impl BufRead, key: String) -> Result<bool, LsmError> {
    todo!("")
}

/// Returns the bloom filters in an sstable
pub(crate) fn get_bloom_filters(buffer: impl BufRead) -> Result<BloomFilterPair, LsmError> {
    todo!("")
}

/// Gets a value from an sstable, can fail if that value doesn't exist in it
pub(crate) fn get_from_table(buffer: impl BufRead, id: i64, key: String) -> Result<Vec<u8>, LsmError> {
    todo!("")
}
