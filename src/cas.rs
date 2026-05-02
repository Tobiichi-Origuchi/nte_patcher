use std::path::{Path, PathBuf};

pub struct BucketManager {
    pub bucket_dir: PathBuf,
}

impl BucketManager {
    pub fn new(bucket_dir: impl AsRef<Path>) -> Self {
        Self {
            bucket_dir: bucket_dir.as_ref().to_path_buf(),
        }
    }

    pub fn get_bucket_path(&self, md5: &str, size: u64) -> PathBuf {
        let shard = if md5.is_empty() { "0" } else { &md5[0..1] };
        self.bucket_dir
            .join(shard)
            .join(format!("{}.{}", md5, size))
    }

    pub fn get_tmp_path(&self, md5: &str, size: u64) -> PathBuf {
        let shard = md5.get(0..1).unwrap_or("0");
        // suffix with timestamp to avoid collisions
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        self.bucket_dir
            .join(shard)
            .join(format!("tmp.{}.{}.{}", md5, size, ts))
    }
}

#[cfg(unix)]
pub async fn create_symlink(original: &Path, link: &Path) -> std::io::Result<()> {
    tokio::fs::symlink(original, link).await
}
#[cfg(windows)]
pub async fn create_symlink(original: &Path, link: &Path) -> std::io::Result<()> {
    tokio::fs::symlink_file(original, link).await
}
