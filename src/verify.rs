#![allow(missing_docs)]
use crate::error::Error;
use md5::{Digest, Md5};
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
        let file = match std::fs::File::open(&path_buf) {
            Ok(f) => f,
            Err(_) => return Ok(false),
        };
        
        let metadata = match file.metadata() {
            Ok(m) => m,
            Err(_) => return Ok(false),
        };

        if metadata.len() == 0 {
            let empty_md5 = Md5::digest([]);
            return Ok(empty_md5.as_slice() == expected_bytes);
        }

        // SAFETY: The file might be modified concurrently by other processes, but this is an accepted risk for zero-copy hashing.
        let mmap = unsafe { memmap2::Mmap::map(&file)? };
        let hash = Md5::digest(&mmap);
        Ok(hash.as_slice() == expected_bytes)
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
        let file = match std::fs::File::open(&path_buf) {
            Ok(f) => f,
            Err(_) => return Ok(false),
        };

        let metadata = match file.metadata() {
            Ok(m) => m,
            Err(_) => return Ok(false),
        };

        if metadata.len() < start + size {
            return Ok(false);
        }

        if size == 0 {
            let empty_md5 = Md5::digest([]);
            return Ok(empty_md5.as_slice() == expected_bytes);
        }

        // SAFETY: Mapping specific slice of file. External modification risk accepted.
        let mmap = unsafe { 
            memmap2::MmapOptions::new().offset(start).len(size as usize).map(&file)? 
        };
        
        let hash = Md5::digest(&mmap);
        Ok(hash.as_slice() == expected_bytes)
    })
    .await
    .unwrap_or(Ok(false))?;

    Ok(is_match)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_file_md5() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.txt");
        std::fs::File::create(&path).unwrap();
        
        // MD5 of empty string is d41d8cd98f00b204e9800998ecf8427e
        let res = check_file_md5(&path, "d41d8cd98f00b204e9800998ecf8427e").await.unwrap();
        assert!(res);
    }
}
