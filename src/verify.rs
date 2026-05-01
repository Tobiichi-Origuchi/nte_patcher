use crate::error::Error;
use md5::{Digest, Md5};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

fn parse_expected_md5(hex_str: &str) -> Option<[u8; 16]> {
    let mut bytes = [0u8; 16];
    hex::decode_to_slice(hex_str, &mut bytes).ok()?;
    Some(bytes)
}

pub async fn check_file_md5(path: &Path, expected_md5: &str) -> Result<bool, Error> {
    let expected_bytes = match parse_expected_md5(expected_md5) {
        Some(b) => b,
        None => return Ok(false),
    };

    let path_buf = path.to_path_buf();

    let is_match = tokio::task::spawn_blocking(move || -> Result<bool, std::io::Error> {
        let mut file = match std::fs::File::open(&path_buf) {
            Ok(f) => f,
            Err(_) => return Ok(false),
        };
        let mut hasher = Md5::new();
        let mut buffer = [0; 131_072];

        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        Ok(hasher.finalize().as_slice() == expected_bytes)
    })
    .await
    .unwrap_or(Ok(false))?;

    Ok(is_match)
}

pub async fn check_slice_md5(
    path: &Path,
    start: u64,
    size: u64,
    expected_md5: &str,
) -> Result<bool, Error> {
    let expected_bytes = match parse_expected_md5(expected_md5) {
        Some(b) => b,
        None => return Ok(false),
    };

    let path_buf = path.to_path_buf();

    let is_match = tokio::task::spawn_blocking(move || -> Result<bool, std::io::Error> {
        let mut file = match std::fs::File::open(&path_buf) {
            Ok(f) => f,
            Err(_) => return Ok(false),
        };

        if file.metadata()?.len() < start + size {
            return Ok(false);
        }

        file.seek(SeekFrom::Start(start))?;

        let mut hasher = Md5::new();
        let mut buffer = [0; 131_072];
        let mut remaining = size;

        while remaining > 0 {
            let to_read = std::cmp::min(remaining, buffer.len() as u64) as usize;
            let n = file.read(&mut buffer[..to_read])?;
            if n == 0 {
                return Ok(false);
            }
            hasher.update(&buffer[..n]);
            remaining -= n as u64;
        }

        Ok(hasher.finalize().as_slice() == expected_bytes)
    })
    .await
    .unwrap_or(Ok(false))?;

    Ok(is_match)
}
