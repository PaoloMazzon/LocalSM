use serde::{Deserialize, Serialize};
use crate::ilog::sstable::EntryLocation;

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

#[cfg(test)]
mod tests {
    use crate::ilog::sstable::EntryLocation;
    use crate::ilog::util::SparseKeyIdTable;

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
}