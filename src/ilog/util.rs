use std::cmp::Ordering;
use std::process::id;
use serde::{Deserialize, Serialize};
use zstd_rs::{CompressionConfig, Compressor, Decompressor};
use crate::ilog::ilog::ILog;
use crate::util::error::Result;

/// Just an entry's key/id/location in file for SparseKeyIdTable
#[derive(Serialize, Deserialize, Debug, PartialOrd, PartialEq, Eq)]
pub(crate) struct EntryLocation {
    /// Key for this record
    pub key: String,

    /// Sort key for this record
    pub sort_key: i64,

    /// How many bytes into the file from the top of the header this record is
    pub location: u64,
}

impl Ord for EntryLocation {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.key > other.key || (self.key == other.key && self.sort_key > other.sort_key) {
            return Ordering::Greater
        } else if self.key < other.key || (self.key == other.key && self.sort_key < other.sort_key) {
            return Ordering::Less
        }
        Ordering::Equal
    }
}

/// An abstraction to make finding occurrences easier to implement
#[derive(PartialOrd, PartialEq, Eq, Clone, Debug)]
struct EntryIdentifier {
    key: String,
    sort_key: Option<i64>
}

impl From<&EntryLocation> for EntryIdentifier {
    fn from(value: &EntryLocation) -> Self {
        Self {
            key: value.key.clone(),
            sort_key: Some(value.sort_key),
        }
    }
}

impl Ord for EntryIdentifier {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.key > other.key {
            return Ordering::Greater
        } else if self.key < other.key {
            return Ordering::Less
        } else if let Some(self_sort) = self.sort_key && let Some(other_sort) = other.sort_key {
            if self_sort > other_sort {
                return Ordering::Greater
            } else if self_sort < other_sort {
                return Ordering::Less
            }
        }
        Ordering::Equal
    }
}

/// Contains a (sorted) list of key/id pairs and their locations in the file.
/// This does not contain all the keys, but can be used to get to right
/// ballpark quickly.
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct SparseKeyIdTable {
    pub key_ids: Vec<EntryLocation>,
}

impl SparseKeyIdTable {
    /// If you are searching for the lowest instance of a given primary key, you would use this to
    /// find the first block you should start searching from (in reverse, since the SSTable files
    /// are sorted in descending order). This function returns an offset where the size of the block
    /// starts, or None if the key cannot be in this table.
    pub fn start_search_from_ascending(&self, key: &str, sort_key: Option<i64>) -> Option<u64> {
        let identifier = EntryIdentifier {
            key: key.to_string(),
            sort_key,
        };

        // Check if the identifier is outside the range of the table
        if identifier.cmp(&self.key_ids.first()?.into()) == Ordering::Greater || identifier.cmp(&self.key_ids.last()?.into()) == Ordering::Less {
            return None
        }

        let mut last_entry = self.key_ids.last()?;
        for record in self.key_ids.iter().rev() {
            match identifier.cmp(&record.into()) {
                Ordering::Equal | Ordering::Less => return Some(last_entry.location),
                Ordering::Greater => (),
            }
            last_entry = record;
        }

        None
    }

    /// If you are searching for the highest instance of a given primary key, you would use this to
    /// find the first compression block to start searching from. Because SSTables are organized in
    /// descending order, you are guaranteed to either find it in the block this points to, or not
    /// find it at all in the SSTable. This function returns an offset where the size of the block
    /// starts (from the top of the header), or None if the key cannot be in this table.
    pub fn start_search_from_descending(&self, key: &str, sort_key: Option<i64>) -> Option<u64> {
        let identifier = EntryIdentifier {
            key: key.to_string(),
            sort_key,
        };

        // Check if the identifier is outside the range of the table
        if identifier.cmp(&self.key_ids.first()?.into()) == Ordering::Greater || identifier.cmp(&self.key_ids.last()?.into()) == Ordering::Less {
            return None
        }

        let mut last_entry = self.key_ids.first()?;
        for record in &self.key_ids[1..] {
            match identifier.cmp(&record.into()) {
                Ordering::Equal | Ordering::Greater => return Some(last_entry.location),
                Ordering::Less => (),
            }
            last_entry = record;
        }

        None
    }

    /// Returns the number of entries in the sparse key-id table
    pub fn len(&self) -> usize {
        self.key_ids.len()
    }
}

/// Helps manage compressing and loading blocks in the SSTable
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct CompressionBlock {
    pub logs: Vec<ILog>,

    /// Estimated size without compression
    #[serde(skip)]
    estimated_size: u64,
}

impl Default for CompressionBlock {
    fn default() -> Self {
        Self {
            logs: Vec::new(),
            estimated_size: 0,
        }
    }
}

impl CompressionBlock {
    /// Creates an empty compression block for you to fill in
    pub fn new() -> Self {
        Self::default()
    }

    /// Attempts to decompress a block and return it
    pub fn decompress(bytes: &[u8]) -> Result<Self> {
        let mut decompressed = Vec::new();
        Decompressor::new().decompress(bytes, None, 1 << 20, &mut decompressed)?;
        Ok(postcard::from_bytes(decompressed.as_slice())?)
    }

    /// Adds an ilog to the internal block. This is expected to already be sorted. If you don't use
    /// this function and instead add directly to logs, size will not be tracked.
    pub fn add_to_block(&mut self, log: ILog) {
        self.estimated_size += (log.key.len() + log.value.len() + log.timestamp.len() + 8 + 1) as u64;
        self.logs.push(log);
    }

    /// Returns a rough estimated size of this block without compression
    pub fn size(&self) -> u64 {
        self.estimated_size
    }

    /// Compresses a block and returns its form as a u8 vector
    pub fn compress(&self) -> Result<Vec<u8>> {
        let serial = postcard::to_allocvec(self)?;
        let mut compressed = Vec::new();
        Compressor::new(CompressionConfig::DEFAULT)?
            .compress(serial.as_slice(), None, &mut compressed)?;
        Ok(compressed)
    }
}

#[cfg(test)]
mod tests {
    use std::iter::zip;
    use crate::ilog::ilog::ILog;
    use super::*;
    use crate::ilog::util::{CompressionBlock, SparseKeyIdTable};

    fn get_test_table() -> SparseKeyIdTable {
        SparseKeyIdTable {
            key_ids: vec![
                EntryLocation {
                    key: "8".to_string(),
                    sort_key: 9,
                    location: 0,
                },
                EntryLocation {
                    key: "8".to_string(),
                    sort_key: 1,
                    location: 1,
                },
                EntryLocation {
                    key: "6".to_string(),
                    sort_key: 1,
                    location: 2,
                },
                EntryLocation {
                    key: "4".to_string(),
                    sort_key: 1,
                    location: 3,
                },
                EntryLocation {
                    key: "1".to_string(),
                    sort_key: 1,
                    location: 4,
                },
            ],
        }
    }

    #[test]
    fn test_start_search_from_descending() {
        // 8/9 8/1 6/1 4/1 1/1
        let table = get_test_table();
        // Tests for the entries not in the table
        assert_eq!(table.start_search_from_descending("2", None), Some(3), "Failed to find an entry in a SparseKeyIdTable");
        assert_eq!(table.start_search_from_descending("3", None), Some(3), "Failed to find an entry in a SparseKeyIdTable");
        assert_eq!(table.start_search_from_descending("5", None), Some(2), "Failed to find an entry in a SparseKeyIdTable");
        assert_eq!(table.start_search_from_descending("7", None), Some(1), "Failed to find an entry in a SparseKeyIdTable");

        // More targeted/edge case
        assert_eq!(table.start_search_from_descending("1", None), Some(3), "Failed to find the very first entry in a SparseKeyIdTable");
        assert_eq!(table.start_search_from_descending("4", None), Some(2), "Failed to find a specific entry in a SparseKeyIdTable");
        assert_eq!(table.start_search_from_descending("6", None), Some(1), "Failed to find a specific entry in a SparseKeyIdTable");
        assert_eq!(table.start_search_from_descending("8", None), Some(0), "Failed to find a specific entry in a SparseKeyIdTable");
        assert_eq!(table.start_search_from_descending("8", Some(0)), Some(1), "Failed to find a specific entry in a SparseKeyIdTable");
        assert_eq!(table.start_search_from_descending("9", None), None, "Found an entry where there should not have been one in a SparseKeyIdTable");
        assert_eq!(table.start_search_from_descending("0", None), None, "Found an entry where there should not have been one in a SparseKeyIdTable");
    }

    #[test]
    fn test_compression_block_compression_and_decompression() {
        let mut block = CompressionBlock::new();
        let source_of_truth = vec![
            ILog {
                key: "asd".to_string(),
                sort_key: 0,
                value: vec![1, 2, 3],
                timestamp: "".to_string(),
                tombstone: false,
            },
            ILog {
                key: "sdf".to_string(),
                sort_key: 1,
                value: vec![2, 3, 4],
                timestamp: "".to_string(),
                tombstone: false,
            },
            ILog {
                key: "dfg".to_string(),
                sort_key: 2,
                value: vec![3, 4, 5],
                timestamp: "".to_string(),
                tombstone: false,
            },
        ];

        for log in &source_of_truth {
            block.add_to_block(log.clone());
        }
        let compressed = block.compress().expect("Compression failed");
        let decompressed_block = CompressionBlock::decompress(compressed.as_slice()).expect("Decompression failed");
        assert_eq!(block.logs.len(), decompressed_block.logs.len(), "Different lengths were found in the original and decompressed block.");
        for (element1, element2) in zip(decompressed_block.logs.iter(), block.logs.iter()) {
            assert_eq!(element1, element2, "At least 1 element from the compressed and decompressed block didn't match. \n{:#?}\nvs{:#?}", element1, element2);
        }
    }

    #[test]
    fn test_search_ascending() {
        // 8/9 8/1 6/1 4/1 1/1
        let table = get_test_table();
        assert_eq!(table.start_search_from_ascending("9", None), None, "Ascending block searches are finding the wrong block.");
        assert_eq!(table.start_search_from_ascending("8", None), Some(2), "Ascending block searches are finding the wrong block.");
        assert_eq!(table.start_search_from_ascending("8", Some(5)), Some(1), "Ascending block searches are finding the wrong block.");
        assert_eq!(table.start_search_from_ascending("7", None), Some(2), "Ascending block searches are finding the wrong block.");
        assert_eq!(table.start_search_from_ascending("6", None), Some(3), "Ascending block searches are finding the wrong block.");
        assert_eq!(table.start_search_from_ascending("5", None), Some(3), "Ascending block searches are finding the wrong block.");
        assert_eq!(table.start_search_from_ascending("4", None), Some(4), "Ascending block searches are finding the wrong block.");
        assert_eq!(table.start_search_from_ascending("3", None), Some(4), "Ascending block searches are finding the wrong block.");
        assert_eq!(table.start_search_from_ascending("2", None), Some(4), "Ascending block searches are finding the wrong block.");
        assert_eq!(table.start_search_from_ascending("1", None), Some(4), "Ascending block searches are finding the wrong block.");
        assert_eq!(table.start_search_from_ascending("0", None), None, "Ascending block searches are finding the wrong block.");
    }

    #[test]
    fn test_ordering_for_entry_location() {
        let entry_1 = EntryLocation {
            key: "1".to_string(),
            sort_key: 0,
            location: 0,
        };
        let entry_2 = EntryLocation {
            key: "1".to_string(),
            sort_key: 2,
            location: 0,
        };
        let entry_3 = EntryLocation {
            key: "3".to_string(),
            sort_key: 0,
            location: 0,
        };
        assert_eq!(entry_1.cmp(&entry_1), Ordering::Equal, "Equal ordering is broken.");
        assert_eq!(entry_1.cmp(&entry_2), Ordering::Less, "Less ordering is broken.");
        assert_eq!(entry_3.cmp(&entry_2), Ordering::Greater, "Greater ordering is broken.");
    }
}
