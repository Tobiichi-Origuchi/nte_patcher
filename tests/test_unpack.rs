use nte_patcher::crypto::aes::unpack;
use nte_patcher::crypto::archive::extract;
use nte_patcher::downloader::worker::download_file;
use nte_patcher::model::Config;
use nte_patcher::parser::manifest::get_config;
use reqwest::Url;
use std::error::Error as StdError;
use std::fs::File;
use std::io::Error;
use std::io::{BufReader, Read};

#[tokio::test]
async fn test_unpack() -> Result<(), Box<dyn StdError>> {
    let config = test_get_config().await?;
    let version = config.resversion.to_string();
    test_download_file(&version).await?;
    test_extract()?;
    test_unpack_reslist()?;
    test_unpack_lastdiff()?;
    Ok(())
}

async fn test_get_config() -> Result<Config, Box<dyn StdError>> {
    let url =
        Url::parse("https://yhcdn1.wmupd.com/clientRes/publish_PC/Version/Windows/config.xml")?;
    let config: Config = get_config(url).await?;
    Ok(config)
}

async fn test_download_file(version: &str) -> Result<(), Box<dyn StdError>> {
    let url = format!(
        "https://yhcdn1.wmupd.com/clientRes/publish_PC/Version/Windows/version/{}/ResList.bin.zip",
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
    let key = b"1289@Patcher0000";
    let iv = b"PatcherSDK000000";
    unpack(origin_path, target_path, key, iv)?;
    Ok(())
}

fn test_unpack_lastdiff() -> Result<(), Error> {
    let origin_path = "lastdiff.bin";
    let target_path = "lastdiff.xml";
    let key = b"1289@Patcher0000";
    let iv = b"PatcherSDK000000";
    unpack(origin_path, target_path, key, iv)?;
    Ok(())
}
