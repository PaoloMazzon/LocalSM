/// Higher-level controller that controls both reads and writes going in and out of the system
pub(crate) struct Controller {
    /// All the MemTables in this controller sit behind a MemTableController
    /// for safety reasons.
    memtables: Vec<MemTableController>,

    /// An in-memory map of all the SSTable files on disk
}
