use thiserror::Error;

#[derive(Debug, Error)]
pub enum KvError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Cannot parse command: {0}")]
    InvalidCommand(String),

    #[error("Cannot convert value {0} to {1}")]
    ConvertError(String, &'static str),

    #[error("Cannot process command {0} with table: {1}, key: {2}. Error: {3}")]
    StorageError(String, String, String, String),

    #[error("Failed to encode protobuf message")]
    EncodeError(#[from] prost::EncodeError),

    #[error("Failed to decode protobuf message")]
    DecodeError(#[from] prost::DecodeError),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Failed to access sled db")]
    SledError(#[from] sled::Error),

    #[error("Frame is too large")]
    FrameError,

    #[error("I/O error")]
    IOError(#[from] std::io::Error),

    #[error("Certificate parse error: error to load {0} {1}")]
    CertificateParseError(&'static str, &'static str),

    #[error("TLS error")]
    TlsError(#[from] tokio_rustls::rustls::TLSError),
}
