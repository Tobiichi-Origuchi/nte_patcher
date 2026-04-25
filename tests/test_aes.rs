use nte_patcher::crypto::aes::unpack_reslist;
use std::io::Error;

#[test]
fn test_unpack_reslist() -> Result<(), Error> {
    let origin_path = "ResList.bin";
    let target_path = "ResList.xml";
    let key = b"1289@Patcher0000";
    let iv = b"PatcherSDK000000";
    unpack_reslist(origin_path, target_path, key, iv)?;
    Ok(())
}
