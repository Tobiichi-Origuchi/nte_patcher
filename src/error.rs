use reqwest::StatusCode;
use std::io::ErrorKind;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Checksum mismatch! Expected {expected}, got {actual}")]
    Checksum { expected: String, actual: String },
    
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
