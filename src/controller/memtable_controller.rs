use crate::ilog::memtable::MemTable;
use std::sync::{Arc, Mutex};

/// Controls reads and writes going in and out of a MemTable for race reasons
pub(crate) struct MemTableController {
    /// MemTable in question
    table: MemTable,

    /// How many reads are currently in this MemTable, if its below 0, readers
    /// may no longer read and its about to be destroyed.
    readers: Arc<i32>,

    /// If true, no more writes may be put into this table
    read_only: Arc<bool>,

    /// For guaranteeing only 1 writer at a time, but reads and writes may
    /// happen concurrently.
    write_lock: Mutex<()>,
}
