#![allow(missing_docs)]
use crate::{
    error::Error,
    model::{Config, PatchList, ResList},
};
use quick_xml::de::from_reader;
use reqwest::{Url, get};
use serde::de::DeserializeOwned;
use std::{
    fs::File,
    io::{BufReader, Cursor},
    path::Path,
};

fn parse<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    Ok(from_reader(reader)?)
}

pub async fn get_config(url: Url) -> Result<Config, Error> {
    let response = get(url).await?.bytes().await?;
    Ok(from_reader(Cursor::new(response))?)
}

pub fn get_reslist(path: impl AsRef<Path>) -> Result<ResList, Error> {
    parse(path)
}

pub fn get_lastdiff(path: impl AsRef<Path>) -> Result<PatchList, Error> {
    parse(path)
}
