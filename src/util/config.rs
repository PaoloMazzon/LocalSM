use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    /// Number of bits to use in the bloom filter, aim for 10x entries_per_record
    bloom_filter_bits: u32,

    /// Number of hashes to use in the bloom filter, must be at least 2, 3-10 usually
    bloom_filter_hashes: u32,

    /// Entries per record stored on disk and in memory
    entries_per_record: u32,

    /// Background worker threads, must be at least 1
    worker_threads: u32,
    
    /// How many bytes each record is allowed to be
    record_allowable_size: u32,
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
        }
    }
}