use crate::error::Error;
use md5::{Digest, Md5};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};

pub async fn check_file_md5(path: &Path, expected_md5: &str) -> Result<bool, Error> {
    let mut file = match File::open(path).await {
        Ok(f) => f,
        Err(_) => return Ok(false),
    };
    let mut hasher = Md5::new();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hex::encode(hasher.finalize()) == expected_md5)
}

pub async fn check_slice_md5(
    path: &Path,
    start: u64,
    size: u64,
    expected_md5: &str,
) -> Result<bool, Error> {
    let mut file = match File::open(path).await {
        Ok(f) => f,
        Err(_) => return Ok(false),
    };
    if file.metadata().await?.len() < start + size {
        return Ok(false);
    }
    file.seek(SeekFrom::Start(start)).await?;
    let mut hasher = Md5::new();
    let mut buffer = [0; 65536];
    let mut remaining = size;
    while remaining > 0 {
        let to_read = std::cmp::min(remaining, buffer.len() as u64) as usize;
        let n = file.read(&mut buffer[..to_read]).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        remaining -= n as u64;
    }
    Ok(hex::encode(hasher.finalize()) == expected_md5)
}
