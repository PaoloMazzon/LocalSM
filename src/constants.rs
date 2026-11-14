/// How we know from the filesystem that an SSTable is currently being written out
static IN_PROGRESS_SSTABLE_PREFIX: &str = "ips_";

/// SSTables are renamed to this once we are finished writing so we know its done
static COMPELTED_SSTABLE_PREFIX: &str = "cst_";
