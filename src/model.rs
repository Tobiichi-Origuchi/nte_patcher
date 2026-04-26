use semver::Version;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(rename = "ResVersion")]
    pub version: Version,
    #[serde(rename = "Section")]
    pub section: String,
    #[serde(rename = "ResSize")]
    pub size: u64,
    #[serde(rename = "Extra")]
    pub extra: Extra,
}

#[derive(Deserialize, Debug)]
pub struct Extra {
    #[serde(rename = "diffHash")]
    pub diff_hash: String,
    #[serde(rename = "listHash")]
    pub list_hash: String,
}

/// 根节点与嵌套的资源列表
#[derive(Deserialize)]
pub struct ResList {
    #[serde(rename = "@version")]
    pub version: Option<Version>,
    #[serde(rename = "Res", default)]
    pub resources: Vec<Res>,
    #[serde(rename = "Package")]
    pub package: Option<Package>,
    #[serde(rename = "BaseVersion", default)]
    pub base_versions: Vec<BaseVersion>,
}

/// 独立文件资源
#[derive(Deserialize)]
pub struct Res {
    #[serde(rename = "@filename")]
    pub filename: String,
    #[serde(rename = "@filesize")]
    pub filesize: u64,
    #[serde(rename = "@md5")]
    pub md5: String,
    #[serde(rename = "@blockSize")]
    pub block_size: Option<usize>,
    #[serde(rename = "Block", default)]
    pub blocks: Vec<Block>,
}

/// 大文件分块信息
#[derive(Deserialize)]
pub struct Block {
    #[serde(rename = "@index")]
    pub index: usize,
    #[serde(rename = "@start")]
    pub start: u64,
    #[serde(rename = "@size")]
    pub size: u64,
    #[serde(rename = "@md5")]
    pub md5: String,
}

/// 打包下载集合
#[derive(Deserialize)]
pub struct Package {
    #[serde(rename = "Pak", default)]
    pub paks: Vec<Pak>,
}

/// 单个打包文件（通常是一个合并下载用的压缩包或数据流）
#[derive(Deserialize)]
pub struct Pak {
    #[serde(rename = "@md5")]
    pub md5: String,
    #[serde(rename = "@filesize")]
    pub filesize: u64,
    #[serde(rename = "Entry", default)]
    pub entries: Vec<Entry>,
}

/// 打包文件内部的文件条目
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

/// 历史版本增量包信息
#[derive(Deserialize)]
pub struct BaseVersion {
    #[serde(rename = "@version")]
    pub version: Version,
    #[serde(rename = "@tag")]
    pub tag: String,
    #[serde(rename = "ResList")]
    pub res_list: ResList,
}
