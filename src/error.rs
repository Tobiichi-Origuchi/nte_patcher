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
}
