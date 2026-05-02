#![allow(missing_docs)]
use crate::error::Error;
use aes::{
    Aes128,
    cipher::{BlockCipherDecrypt, KeyInit},
};
use flate2::read::ZlibDecoder;
use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom, copy},
    path::Path,
};

pub fn aes_cbc(
    origin_path: impl AsRef<Path>,
    target_path: impl AsRef<Path>,
    key: &[u8; 16],
    iv: &[u8; 16],
) -> Result<(), Error> {
    let file = File::open(origin_path)?;
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(16))?;

    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    let len = buffer.len();
    if len == 0 || len % 16 != 0 {
        return Err(Error::Validation("Invalid payload length".to_string()));
    }

    let cipher = Aes128::new(key.into());
    let mut prev_iv = *iv;

    for chunk in buffer.chunks_exact_mut(16) {
        let mut cipher_block = [0u8; 16];
        cipher_block.copy_from_slice(chunk);
        let mut block = cipher_block.into();
        cipher.decrypt_block(&mut block);
        let decrypted_block: [u8; 16] = block.into();
        for i in 0..16 {
            chunk[i] = decrypted_block[i] ^ prev_iv[i];
        }

        prev_iv = cipher_block;
    }

    let pad_len = buffer[len - 1] as usize;
    if pad_len == 0 || pad_len > 16 {
        return Err(Error::Validation("Invalid padding".to_string()));
    }
    for &b in &buffer[len - pad_len..] {
        if b as usize != pad_len {
            return Err(Error::Validation("Invalid padding".to_string()));
        }
    }
    buffer.truncate(len - pad_len);

    let mut decoder = ZlibDecoder::new(&buffer[..]);
    let mut target_file = File::create(target_path)?;
    copy(&mut decoder, &mut target_file)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_invalid_payload_length() {
        let key = [0u8; 16];
        let iv = [0u8; 16];
        
        let origin = NamedTempFile::new().unwrap();
        let target = NamedTempFile::new().unwrap();
        
        let mut file = std::fs::File::create(origin.path()).unwrap();
        file.write_all(&[1, 2, 3]).unwrap(); // Invalid length
        
        let res = aes_cbc(origin.path(), target.path(), &key, &iv);
        assert!(matches!(res, Err(Error::Validation(_))));
    }
}
