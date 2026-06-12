#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigDiscoveryError {
    UnknownCoreKey(String),
    InvalidServerRoot(String),
    Io(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigIoError {
    UnsupportedFormat(String),
    ParseFailed(String),
    WriteFailed(String),
    Io(String),
}
