use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    /// Number of bits to use in the bloom filter, aim for 10x entries_per_record
    pub bloom_filter_bits: u32,

    /// Number of hashes to use in the bloom filter, must be at least 2, 3-10 usually
    pub bloom_filter_hashes: u32,

    /// Entries per record stored on disk and in memory
    pub entries_per_record: u32,

    /// Background worker threads, must be at least 1
    pub worker_threads: u32,
    
    /// How many bytes each record is allowed to be
    pub record_allowable_size: u32,
    
    /// Directory where system-files like temporary cache and settings are stored
    pub system_directory: String,
    
    /// Additional directories where SSTables may be stored to, if empty only system_directory is used
    pub data_directory: Vec<String>,
}

impl Default for Config {
    /// Generates some reasonable defaults
    fn default() -> Self {
        let base_thread_count = match std::thread::available_parallelism() {
            Ok(t) => t.get(),
            Err(e) => 1,
        };
        Config {
            bloom_filter_bits: 1000,
            bloom_filter_hashes: 3,
            entries_per_record: 100,
            worker_threads: base_thread_count as u32,
            record_allowable_size: 1024 * 1024 * 10,
            system_directory: "/var/localsm/".to_string(),
            data_directory: Vec::new(),
        }
    }
}