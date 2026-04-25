use nte_patcher::crypto::archive::extract;
use std::fs::File;
use std::io::{BufReader, Error, Read};

#[test]
fn test_extract() -> Result<(), Error> {
    let file = File::open("ResList.bin.zip")?;
    let mut reader = BufReader::new(file);
    let mut data = Vec::new();
    reader.read_to_end(&mut data)?;
    extract(&data, ".")?;
    Ok(())
}
