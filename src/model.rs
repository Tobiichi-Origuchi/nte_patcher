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
    pub diffhash: String,
    #[serde(rename = "listHash")]
    pub listhash: String,
}

#[derive(Deserialize)]
pub struct ResList {
    #[serde(rename = "@version", default)]
    pub version: Option<Version>,
    #[serde(rename = "@tag", default)]
    pub tag: Option<String>,
    #[serde(rename = "Res", default)]
    pub res: Vec<Res>,
    #[serde(rename = "Package")]
    pub package: Option<Package>,
    #[serde(rename = "BaseVersion", default)]
    pub baseversion: Vec<BaseVersion>,
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
    pub blocksize: Option<u8>,
    #[serde(rename = "Block", default)]
    pub block: Vec<Block>,
}

#[derive(Deserialize, Clone)]
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

#[derive(Deserialize, Clone)]
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
    #[serde(rename = "@version", default)]
    pub version: Option<Version>,
    #[serde(rename = "@tag", default)]
    pub tag: Option<String>,
    #[serde(rename = "ResList", default)]
    pub reslist: Option<Box<ResList>>,
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

#[derive(Clone)]
pub enum TaskType {
    Normal,
    Block { blocks: Vec<Block> },
    Pak { entries: Vec<Entry> },
}

#[derive(Clone)]
pub struct ResTask {
    pub target_path: String,
    pub filesize: u64,
    pub md5: String,
    pub task_type: TaskType,
}

impl ResTask {
    pub fn from_reslist(reslist: ResList) -> Vec<Self> {
        let mut tasks = Vec::new();

        for res in reslist.res {
            let task_type = if res.block.is_empty() {
                TaskType::Normal
            } else {
                TaskType::Block { blocks: res.block }
            };

            tasks.push(ResTask {
                target_path: res.filename,
                filesize: res.filesize,
                md5: res.md5,
                task_type,
            });
        }

        if let Some(package) = reslist.package {
            for pak in package.pak {
                tasks.push(ResTask {
                    target_path: format!("{}.pak", pak.md5),
                    filesize: pak.filesize,
                    md5: pak.md5.clone(),
                    task_type: TaskType::Pak { entries: pak.entry },
                });
            }
        }

        tasks
    }
}
