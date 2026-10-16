pub(crate) type Result<T> = std::result::Result<T, LsmError>;

#[derive(Clone, Debug)]
pub enum LsmError {
    /// Binary serialization/deserialization has failed
    BinaryEncodingError(String),

    /// JSON serialization/deserialization has failed
    JsonEncodingError(String),
    
    /// A file was unable to be opened/written to/read from/...
    FileNotAvailable(String),
    
    /// An entry was requested but none was found matching it
    EntryNotPresent,

    /// SSTable was invalid for some reason
    InvalidStringTable(String),

    /// An error with compression or decompression
    CompressionError(String),
    
    /// We don't know what happened, mainly for debugging/development
    Unknown(String),
}

impl ToString for LsmError {
    fn to_string(&self) -> String {
        match self {
            LsmError::BinaryEncodingError(e)
                => format!("Binary encoding error: {}", e),
            LsmError::JsonEncodingError(e)
                => format!("JSON encoding error: {}", e),
            LsmError::FileNotAvailable(e)
                => format!("Filesystem error: {}", e),
            LsmError::EntryNotPresent
                => "Entry not present".to_string(),
            LsmError::InvalidStringTable(e)
                => format!("Invalid string table: {}", e),
            LsmError::CompressionError(e)
                => format!("Compression/Decompression error: {}", e),
            LsmError::Unknown(e)
                => format!("Unknown error: {}", e),
        }
    }
}

impl From<std::io::Error> for LsmError {
    fn from(e: std::io::Error) -> Self {
        LsmError::FileNotAvailable(e.to_string())
    }
}

impl From<serde_json::Error> for LsmError {
    fn from(e: serde_json::Error) -> Self {
        LsmError::JsonEncodingError(e.to_string())
    }
}

impl From<postcard::Error> for LsmError {
    fn from(e: postcard::Error) -> Self {
        LsmError::BinaryEncodingError(e.to_string())
    }
}

impl From<zstd_rs::Error> for LsmError {
    fn from(e: zstd_rs::Error) -> Self {
        LsmError::BinaryEncodingError(e.to_string())
    }
}
