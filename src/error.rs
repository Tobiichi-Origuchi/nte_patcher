use reqwest::StatusCode;
use std::io::ErrorKind;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("XML deserialization error: {0}")]
    Xml(#[from] quick_xml::DeError),
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("URL parsing error: {0}")]
    Url(#[from] url::ParseError),
    #[error("Payload length is not a multiple of 16")]
    InvalidPayload,
    #[error("Invalid or corrupted PKCS7 padding")]
    InvalidPadding,
    #[error("MD5 mismatch! Expected {expected}, got {actual}")]
    Md5Mismatch { expected: String, actual: String },
}

impl Error {
    pub fn is_retryable(&self) -> bool {
        match self {
            Error::Reqwest(e) => {
                if let Some(status) = e.status() {
                    status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS
                } else {
                    true
                }
            }

            Error::Md5Mismatch { .. } => true,

            Error::Io(e) => matches!(
                e.kind(),
                ErrorKind::ConnectionAborted
                    | ErrorKind::ConnectionReset
                    | ErrorKind::ConnectionRefused
                    | ErrorKind::TimedOut
                    | ErrorKind::Interrupted
                    | ErrorKind::UnexpectedEof
            ),

            Error::Xml(_)
            | Error::Zip(_)
            | Error::Url(_)
            | Error::InvalidPayload
            | Error::InvalidPadding => false,
        }
    }
}
