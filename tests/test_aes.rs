use nte_patcher::crypto::aes::unpack;
use std::io::Error;

#[test]
fn test_unpack_reslist() -> Result<(), Error> {
    let origin_path = "ResList.bin";
    let target_path = "ResList.xml";
    let key = b"1289@Patcher0000";
    let iv = b"PatcherSDK000000";
    unpack(origin_path, target_path, key, iv)?;
    Ok(())
}

#[test]
fn test_unpack_lastdiff() -> Result<(), Error> {
    let origin_path = "lastdiff.bin";
    let target_path = "lastdiff.xml";
    let key = b"1289@Patcher0000";
    let iv = b"PatcherSDK000000";
    unpack(origin_path, target_path, key, iv)?;
    Ok(())
}
