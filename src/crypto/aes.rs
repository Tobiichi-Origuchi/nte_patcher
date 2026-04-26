use aes::Aes128;
use aes::cipher::{KeyIvInit, block_padding::Pkcs7};
use cbc::Decryptor;
use cipher::BlockModeDecrypt;
use flate2::read::ZlibDecoder;
use std::fs::File;
use std::io::{BufReader, Error, Read, Seek, SeekFrom, Write};
use std::path::Path;

type Aes128CbcDec = Decryptor<Aes128>;

fn get_payload(path: impl AsRef<Path>) -> Result<Vec<u8>, Error> {
    let file = File::open(path)?;
    let mut payload = Vec::new();
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(16))?;
    reader.read_to_end(&mut payload)?;
    Ok(payload)
}

fn decrypt_payload(payload: Vec<u8>, key: &[u8; 16], iv: &[u8; 16]) -> Result<Vec<u8>, Error> {
    let cipher = Aes128CbcDec::new(key.into(), iv.into());
    let decrypted = cipher.decrypt_padded_vec::<Pkcs7>(&payload).unwrap();
    Ok(decrypted)
}

fn decompress_decrypted(decrypted: Vec<u8>) -> Result<Vec<u8>, Error> {
    let mut decompressed = Vec::new();
    let mut decoder = ZlibDecoder::new(&decrypted[..]);
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

pub fn unpack(
    origin_path: impl AsRef<Path>,
    target_path: impl AsRef<Path>,
    key: &[u8; 16],
    iv: &[u8; 16],
) -> Result<(), Error> {
    let payload = get_payload(origin_path)?;
    let decrypted = decrypt_payload(payload, key, iv)?;
    let decompressed = decompress_decrypted(decrypted)?;
    let mut file = File::create(target_path)?;
    file.write_all(&decompressed)?;
    Ok(())
}
