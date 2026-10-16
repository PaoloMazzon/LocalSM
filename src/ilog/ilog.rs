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
    
}
