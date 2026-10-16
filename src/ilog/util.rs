use serde::{Deserialize, Serialize};
use zstd_rs::{CompressionConfig, Compressor, Decompressor};
use crate::ilog::ilog::ILog;
use crate::ilog::sstable::EntryLocation;
use crate::util::error::Result;

/// Contains a (sorted) list of key/id pairs and their locations in the file.
/// This does not contain all the keys, but can be used to get to right
/// ballpark quickly.
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct SparseKeyIdTable {
    pub key_ids: Vec<EntryLocation>,
}

impl SparseKeyIdTable {
    /// Finds where you should start walking the file. SSTables are sorted in descending order,
    /// such that you find the highest and newest keys first then the older/smaller keys after. This
    /// is so you can find tombstones first.
    pub fn start_search_from(&self, key: &str) -> Option<u64> {
        let mut previous_location = 0;
        let key_string = key.to_string();

        // The requested record is not within the bounds of this table since sparse tables always
        // include the first and last element in the SSTable.
        if key_string < self.key_ids.last()?.key || key_string > self.key_ids.first()?.key {
            return None
        }

        // Find where it should be in reverse order
        for record in &self.key_ids {
            if key_string >= record.key {
                return Some(previous_location)
            }
            previous_location = record.location;
        }

        Some(previous_location)
    }

    /// Returns the number of entries in the sparse key-id table
    pub fn len(&self) -> usize {
        self.key_ids.len()
    }
}

/// Helps manage compressing and loading blocks in the SSTable
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct CompressionBlock {
    pub logs: Vec<ILog>
}

impl CompressionBlock {
    /// Creates an empty compression block for you to fill in
    pub fn new() -> Self {
        Self {
            logs: Vec::new()
        }
    }

    /// Attempts to decompress a block and return it
    pub fn decompress(bytes: &[u8]) -> Result<Self> {
        let mut decompressed = Vec::new();
        Decompressor::new().decompress(bytes, None, 1 << 20, &mut decompressed)?;
        Ok(postcard::from_bytes(decompressed.as_slice())?)
    }

    /// Adds an ilog to the internal block. This is expected to already be sorted.
    pub fn add_to_block(&mut self, log: ILog) {
        self.logs.push(log);
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
    use crate::ilog::sstable::EntryLocation;
    use crate::ilog::util::{CompressionBlock, SparseKeyIdTable};

    #[test]
    fn test_start_search_from() {
        let table = SparseKeyIdTable {
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
        };
        // Tests for the entries not in the table
        assert_eq!(table.start_search_from("2").unwrap(), 3, "Failed to find an entry in a SparseKeyIdTable");
        assert_eq!(table.start_search_from("3").unwrap(), 3, "Failed to find an entry in a SparseKeyIdTable");
        assert_eq!(table.start_search_from("5").unwrap(), 2, "Failed to find an entry in a SparseKeyIdTable");
        assert_eq!(table.start_search_from("7").unwrap(), 1, "Failed to find an entry in a SparseKeyIdTable");

        // More targeted/edge case
        assert_eq!(table.start_search_from("1").unwrap(), 3, "Failed to find the very first entry in a SparseKeyIdTable");
        assert_eq!(table.start_search_from("4").unwrap(), 2, "Failed to find a specific entry in a SparseKeyIdTable");
        assert_eq!(table.start_search_from("6").unwrap(), 1, "Failed to find a specific entry in a SparseKeyIdTable");
        assert_eq!(table.start_search_from("8").unwrap(), 0, "Failed to find a specific entry in a SparseKeyIdTable");
        assert_eq!(table.start_search_from("9"), None, "Found an entry where there should not have been one in a SparseKeyIdTable");
        assert_eq!(table.start_search_from("0"), None, "Found an entry where there should not have been one in a SparseKeyIdTable");
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
}