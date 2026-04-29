use std::{
    fs::{File, create_dir_all},
    io::{Cursor, Error, copy},
    path::Path,
};
use zip::ZipArchive;

pub fn extract(data: &[u8], base_path: impl AsRef<Path>) -> Result<(), Error> {
    let base_path = base_path.as_ref();
    if !base_path.exists() {
        create_dir_all(base_path)?;
    };

    let cursor = Cursor::new(data);
    let mut archive = ZipArchive::new(cursor)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        // Prevent Zip Slip
        let outpath = match file.enclosed_name() {
            Some(path) => base_path.join(path),
            None => continue,
        };

        if file.is_file() {
            let mut outfile = File::create(&outpath)?;
            copy(&mut file, &mut outfile)?;
        }
    }
    Ok(())
}
