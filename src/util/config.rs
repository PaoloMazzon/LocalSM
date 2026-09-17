use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    bloom_field_bits: u32
}

impl Default for Config {
    fn default() -> Self {
        Config {
            bloom_field_bits: 32
        }
    }
}