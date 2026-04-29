use crate::{
    error::Error,
    model::{Config, PatchList, ResList},
};
use quick_xml::de::from_reader;
use reqwest::{Url, get};
use std::{
    fs::File,
    io::{BufReader, Cursor},
    path::Path,
};

pub async fn get_config(url: Url) -> Result<Config, Error> {
    let response = get(url).await?.bytes().await?;
    let cursor = Cursor::new(response);
    let config: Config = from_reader(cursor)?;
    Ok(config)
}

pub fn get_reslist(path: impl AsRef<Path>) -> Result<ResList, Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let reslist: ResList = from_reader(reader)?;
    Ok(reslist)
}

pub fn get_lastdiff(path: impl AsRef<Path>) -> Result<PatchList, Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let reslist: PatchList = from_reader(reader)?;
    Ok(reslist)
}
