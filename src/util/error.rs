#[derive(Clone, Debug)]
pub enum LsmError {
    BinaryEncodingError(String),
    FileNotAvailable(String),
}


