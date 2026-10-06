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
