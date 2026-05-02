//! Error types and error handling logic for the SDK.

use reqwest::StatusCode;
use std::io::ErrorKind;
use thiserror::Error;

/// Represents all possible errors that can occur during patching operations.
#[derive(Error, Debug)]
pub enum Error {
    /// A standard I/O error occurred (e.g., file not found, permission denied).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// A network error occurred during an HTTP request.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    /// The calculated checksum of a downloaded file or chunk did not match the expected value.
    #[error("Checksum mismatch! Expected {expected}, got {actual}")]
    Checksum {
        /// The expected MD5 hash in hex format.
        expected: String,
        /// The actual calculated MD5 hash in hex format.
        actual: String
    },
    
    /// A validation or parsing error occurred (e.g., invalid payload, XML syntax error).
    #[error("Validation error: {0}")]
    Validation(String),
}

impl From<quick_xml::DeError> for Error {
    fn from(e: quick_xml::DeError) -> Self {
        Self::Validation(format!("XML error: {}", e))
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(e: zip::result::ZipError) -> Self {
        Self::Validation(format!("Zip error: {}", e))
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Self::Validation(format!("URL parsing error: {}", e))
    }
}

impl Error {
    /// Determines whether the error might be resolved by retrying the operation.
    pub fn is_retryable(&self) -> bool {
        match self {
            Error::Network(e) => {
                if let Some(status) = e.status() {
                    status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS || status == StatusCode::REQUEST_TIMEOUT
                } else {
                    true
                }
            }
            Error::Checksum { .. } => true,
            Error::Io(e) => matches!(
                e.kind(),
                ErrorKind::ConnectionAborted
                    | ErrorKind::ConnectionReset
                    | ErrorKind::ConnectionRefused
                    | ErrorKind::TimedOut
                    | ErrorKind::Interrupted
                    | ErrorKind::UnexpectedEof
            ),
            Error::Validation(_) => false,
        }
    }
}
