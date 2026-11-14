use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct SsTableConfig {
    /// Number of bits to use in the bloom filter, aim for 10x entries_per_record
    pub bloom_filter_bits: u32,

    /// Number of hashes to use in the bloom filter, must be at least 2, 3-10 usually
    pub bloom_filter_hashes: u32,

    /// General ball-park for how large (in bytes) compression blocks should be (before compression)
    pub size_of_compression_blocks: u32,

    /// Even if the block is not quite size_of_compression_blocks yet, if this number is hit a new compression block is started
    pub maximum_records_per_compression_block: u32,

    /// How many bytes each record is allowed to be
    pub record_allowable_size: u32,
}

impl Default for SsTableConfig {
    fn default() -> Self {
        SsTableConfig {
            bloom_filter_bits: 1000,
            bloom_filter_hashes: 3,
            size_of_compression_blocks: 1024 * 1024 * 16, // 16MiB
            maximum_records_per_compression_block: 100,
            record_allowable_size: 1024 * 1024 * 10,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SystemConfig {
    /// Background worker threads, must be at least 1
    pub worker_threads: u32,

    /// Directory where system-files like temporary cache and settings are stored
    pub system_directory: String,

    /// Directory where SSTables may be stored to, if empty system_directory is used
    pub data_directory: String,
}

impl Default for SystemConfig {
    fn default() -> Self {
        let base_thread_count = match std::thread::available_parallelism() {
            Ok(t) => t.get(),
            Err(_) => 1,
        };
        SystemConfig {
            worker_threads: base_thread_count as u32,
            system_directory: "/var/localsm/".to_string(),
            data_directory: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    /// Config specific to the optimization of SSTables
    pub sstable_config: SsTableConfig,

    /// Global/system config
    pub system_config: SystemConfig,
}

impl Default for Config {
    /// Generates some reasonable defaults
    fn default() -> Self {
        Config {
            sstable_config: SsTableConfig::default(),
            system_config: SystemConfig::default(),
        }
    }
}
