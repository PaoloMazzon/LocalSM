use std::cmp::Ordering;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd)]
pub(crate) struct ILog {
    /// String key for this one
    pub key: String,

    /// 64-bit integer that can be used to sort records
    pub sort_key: i64,

    /// Payload in raw binary
    pub value: Vec<u8>,

    /// ISO-8601 formatted string
    pub timestamp: String,

    /// If this is a deletion record
    pub tombstone: bool,
}

impl Ord for ILog {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.key < other.key {
            return Ordering::Less
        } else if self.key > other.key {
            return Ordering::Greater
        }
        if self.sort_key < other.sort_key {
            return Ordering::Less
        } else if self.sort_key > other.sort_key {
            return Ordering::Greater
        }
        let timestamp_x = match chrono::DateTime::parse_from_rfc3339(self.timestamp.as_str()) {
            Ok(t) => t,
            Err(_) => return Ordering::Equal,
        };
        let timestamp_y = match chrono::DateTime::parse_from_rfc3339(other.timestamp.as_str()) {
            Ok(t) => t,
            Err(_) => return Ordering::Equal,
        };
        if timestamp_x < timestamp_y {
            return Ordering::Less
        } else if timestamp_x > timestamp_y {
            return Ordering::Greater
        }
        Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;
    use crate::ilog::ilog::ILog;

    #[test]
    fn test_ordering_for_ilog() {
        let ilog_9_1_2020 = ILog {
            key: "9".to_string(),
            sort_key: 1,
            value: vec![0, 1],
            timestamp: "2020-01-01T00:00:00-00:00".to_string(),
            tombstone: false,
        };
        let ilog_1_3_2020 = ILog {
            key: "1".to_string(),
            sort_key: 3,
            value: vec![0, 1],
            timestamp: "2020-01-01T00:00:00-00:00".to_string(),
            tombstone: false,
        };
        let ilog_9_2_2020 = ILog {
            key: "9".to_string(),
            sort_key: 2,
            value: vec![0, 1],
            timestamp: "2020-01-01T00:00:00-00:00".to_string(),
            tombstone: false,
        };
        let ilog_1_3_2021 = ILog {
            key: "1".to_string(),
            sort_key: 3,
            value: vec![0, 1],
            timestamp: "2021-01-01T00:00:00-00:00".to_string(),
            tombstone: false,
        };
        // First make sure the strings are ISO 8601
        chrono::DateTime::parse_from_rfc3339(ilog_9_1_2020.timestamp.as_str()).unwrap();
        chrono::DateTime::parse_from_rfc3339(ilog_1_3_2020.timestamp.as_str()).unwrap();
        chrono::DateTime::parse_from_rfc3339(ilog_9_2_2020.timestamp.as_str()).unwrap();
        chrono::DateTime::parse_from_rfc3339(ilog_1_3_2021.timestamp.as_str()).unwrap();

        // Primary key ordering
        assert_eq!(ilog_9_1_2020.cmp(&ilog_1_3_2020), Ordering::Greater, "");
        assert_eq!(ilog_1_3_2020.cmp(&ilog_9_1_2020), Ordering::Less, "");

        // Equality
        assert_eq!(ilog_9_1_2020.cmp(&ilog_9_1_2020), Ordering::Equal, "");

        // Secondary key ordering
        assert_eq!(ilog_9_2_2020.cmp(&ilog_9_1_2020), Ordering::Greater, "");
        assert_eq!(ilog_9_1_2020.cmp(&ilog_9_2_2020), Ordering::Less, "");

        // Time-based ordering
        assert_eq!(ilog_1_3_2021.cmp(&ilog_1_3_2020), Ordering::Greater, "");
        assert_eq!(ilog_1_3_2020.cmp(&ilog_1_3_2021), Ordering::Less, "");
    }
}
