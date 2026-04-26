use semver::Version;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    #[serde(rename = "ResVersion")]
    pub resversion: Version,
    #[serde(rename = "Section")]
    pub section: String,
    #[serde(rename = "ResSize")]
    pub size: u64,
    #[serde(rename = "Extra")]
    pub extra: Extra,
}

#[derive(Deserialize)]
pub struct Extra {
    #[serde(rename = "diffHash")]
    pub diff_hash: String,
    #[serde(rename = "listHash")]
    pub list_hash: String,
}

#[derive(Deserialize)]
pub struct ResList {
    #[serde(rename = "@version")]
    pub version: Version,
    #[serde(rename = "@tag")]
    pub tag: String,
    #[serde(rename = "Res")]
    pub res: Vec<Res>,
    #[serde(rename = "Package")]
    pub package: Option<Package>,
    #[serde(rename = "BaseVersion")]
    pub base_version: Vec<BaseVersion>,
}

#[derive(Deserialize)]
pub struct Res {
    #[serde(rename = "@filename")]
    pub filename: String,
    #[serde(rename = "@filesize")]
    pub filesize: u64,
    #[serde(rename = "@md5")]
    pub md5: String,
    #[serde(rename = "@blockSize")]
    pub block_size: Option<u8>,
    #[serde(rename = "Block")]
    pub block: Vec<Block>,
}

#[derive(Deserialize)]
pub struct Block {
    #[serde(rename = "@index")]
    pub index: u8,
    #[serde(rename = "@start")]
    pub start: u64,
    #[serde(rename = "@size")]
    pub size: u64,
    #[serde(rename = "@md5")]
    pub md5: String,
}

#[derive(Deserialize)]
pub struct Package {
    #[serde(rename = "Pak")]
    pub pak: Vec<Pak>,
}

#[derive(Deserialize)]
pub struct Pak {
    #[serde(rename = "@md5")]
    pub md5: String,
    #[serde(rename = "@filesize")]
    pub filesize: u64,
    #[serde(rename = "Entry")]
    pub entry: Vec<Entry>,
}

#[derive(Deserialize)]
pub struct Entry {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@start")]
    pub start: u64,
    #[serde(rename = "@offset")]
    pub offset: u64,
    #[serde(rename = "@size")]
    pub size: u64,
    #[serde(rename = "@md5")]
    pub md5: String,
    #[serde(rename = "@check")]
    pub check: Option<u8>,
}

#[derive(Deserialize)]
pub struct BaseVersion {
    #[serde(rename = "@version")]
    pub version: Version,
    #[serde(rename = "@tag")]
    pub tag: String,
    #[serde(rename = "ResList")]
    pub res_list: ResList,
}

#[derive(Deserialize)]
pub struct PatchList {
    #[serde(rename = "Patch")]
    pub patch: Vec<Patch>,
    #[serde(rename = "Section")]
    pub section: Vec<Section>,
}

#[derive(Deserialize)]
pub struct Patch {
    #[serde(rename = "@oldfile")]
    pub oldfile: String,
    #[serde(rename = "@newfile")]
    pub newfile: String,
    #[serde(rename = "@patch")]
    pub patch: String,
    #[serde(rename = "@v")]
    pub v: String,
    #[serde(rename = "Block")]
    pub block: Vec<Block>,
}

#[derive(Deserialize)]
pub struct Section {
    #[serde(rename = "@resversion")]
    pub resversion: Version,
    #[serde(rename = "Patch")]
    pub patch: Vec<Patch>,
}
