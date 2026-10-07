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
    
    /// We don't know what happened, mainly for debugging/development
    Unknown(String),
}

impl From<std::io::Error> for LsmError {
    fn from(e: std::io::Error) -> Self {
        LsmError::FileNotAvailable(format!("{:?}", e))
    }
}

impl From<serde_json::Error> for LsmError {
    fn from(e: serde_json::Error) -> Self {
        LsmError::JsonEncodingError(format!("{:?}", e))
    }
}

impl From<serde_binary::Error> for LsmError {
    fn from(e: serde_binary::Error) -> Self {
        LsmError::BinaryEncodingError(format!("{:?}", e))
    }
}
