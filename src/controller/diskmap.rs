use chrono::{DateTime, Utc};

/// Simple abstraction to scan directories
struct TableList {
    /// List of the absolute filenames
    tables: Vec<String>,

    /// Last time this list was refreshed
    last_refresh_time: DateTime<Utc>,

    /// Root directory, absolute path
    root_directory: String,
}

impl TableList {
    pub fn new(root_directory: String) -> Self {
        todo!("");
    }
}


/// Map of contents on the filesystem so we can cache and refresh with a simple
/// abstraction.
pub(crate) struct DiskMap {
    /// The levels of the sstables, L0, L1, ...
    levels: Vec<TableList>,
}
