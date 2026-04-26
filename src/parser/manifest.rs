use crate::model::Config;
use quick_xml::de::from_reader;
use reqwest::Url;
use std::io::Cursor;

pub async fn get_config(url: Url) -> Result<Config, Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?.bytes().await?;
    let cursor = Cursor::new(response);
    let config: Config = from_reader(cursor)?;
    Ok(config)
}
