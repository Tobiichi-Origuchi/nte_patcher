use md5::{Digest, Md5};
use nte_patcher::{
    crypto::aes_cbc, download::download_file, error::Error, model::Config, parser::get_config,
    unzip::extract,
};
use reqwest::Url;
use std::{
    fs::{File, read},
    io::{BufReader, Read},
};

fn md5(file_path: &str) -> Result<String, Error> {
    let data = read(file_path)?;
    let mut hasher = Md5::new();
    hasher.update(&data);
    let result = hasher.finalize();
    let hex = hex::encode(result);
    Ok(hex)
}

#[tokio::test]
async fn test_unpack() -> Result<(), Error> {
    let config = test_get_config().await?;
    let version = config.resversion.to_string();
    let diffhash = config.extra.diffhash;
    let listhash = config.extra.listhash;
    test_download_file(&version).await?;
    test_extract()?;
    test_unpack_reslist()?;
    test_unpack_lastdiff()?;
    let diffhash_actual = md5("lastdiff.xml")?;
    let listhash_actual = md5("ResList.xml")?;
    assert_eq!(diffhash, diffhash_actual);
    assert_eq!(listhash, listhash_actual);
    Ok(())
}

async fn test_get_config() -> Result<Config, Error> {
    let url =
        Url::parse("https://ntecdn1.wmupd.com/clientRes/publish_PC/Version/Windows/config.xml")?;
    let config: Config = get_config(url).await?;
    Ok(config)
}

async fn test_download_file(version: &str) -> Result<(), Error> {
    let url = format!(
        "https://ntecdn1.wmupd.com/clientRes/publish_PC/Version/Windows/version/{}/ResList.bin.zip",
        version
    );
    let path = "ResList.bin.zip";
    download_file(&url, path).await?;
    Ok(())
}

fn test_extract() -> Result<(), Error> {
    let file = File::open("ResList.bin.zip")?;
    let mut reader = BufReader::new(file);
    let mut data = Vec::new();
    reader.read_to_end(&mut data)?;
    extract(&data, ".")?;
    Ok(())
}

fn test_unpack_reslist() -> Result<(), Error> {
    let origin_path = "ResList.bin";
    let target_path = "ResList.xml";
    let key = b"3000001@Patcher0";
    let iv = b"PatcherSDK000000";
    aes_cbc(origin_path, target_path, key, iv)?;
    Ok(())
}

fn test_unpack_lastdiff() -> Result<(), Error> {
    let origin_path = "lastdiff.bin";
    let target_path = "lastdiff.xml";
    let key = b"3000001@Patcher0";
    let iv = b"PatcherSDK000000";
    aes_cbc(origin_path, target_path, key, iv)?;
    Ok(())
}
