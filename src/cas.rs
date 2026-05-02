use crate::error::Error;
use futures_util::StreamExt;
use md5::{Digest, Md5};
use reqwest::{Client, header::RANGE};
use std::path::{Path, PathBuf};
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt, SeekFrom};

pub struct BucketManager {
    client: Client,
    pub bucket_dir: PathBuf,
}

impl BucketManager {
    pub fn new(client: Client, bucket_dir: impl AsRef<Path>) -> Self {
        Self {
            client,
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

    pub async fn sync_file<F>(
        &self,
        url: &str,
        target_path: &Path,
        expected_md5: &str,
        expected_size: u64,
        mut on_progress: F,
    ) -> Result<(), Error>
    where
        F: FnMut(u64),
    {
        let bucket_path = self.get_bucket_path(expected_md5, expected_size);
        let tmp_path = self.get_tmp_path(expected_md5, expected_size);

        if let Some(parent) = bucket_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        if target_path.exists() || fs::symlink_metadata(target_path).await.is_ok() {
            if let Ok(p) = fs::read_link(target_path).await {
                // use absolute paths to compare symlink target and bucket path
                let is_same = fs::canonicalize(&p).await.unwrap_or_default()
                    == fs::canonicalize(&bucket_path).await.unwrap_or_default();
                if is_same && bucket_path.exists() {
                    on_progress(expected_size);
                    return Ok(());
                }
            } else {
                fs::remove_file(target_path).await?;
            }
        }

        if !bucket_path.exists() {
            self.download_to_tmp(
                url,
                &tmp_path,
                expected_md5,
                expected_size,
                &mut on_progress,
            )
            .await?;
            fs::rename(&tmp_path, &bucket_path).await?;
        } else {
            on_progress(expected_size);
        }

        create_symlink(&bucket_path, target_path).await?;
        Ok(())
    }

    async fn download_to_tmp<F>(
        &self,
        url: &str,
        tmp_path: &Path,
        expected_md5: &str,
        expected_size: u64,
        on_progress: &mut F,
    ) -> Result<(), Error>
    where
        F: FnMut(u64),
    {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(tmp_path)
            .await?;

        let mut existing_size = file.metadata().await?.len();
        if existing_size > expected_size {
            file.set_len(0).await?;
            existing_size = 0;
        }

        if existing_size < expected_size {
            file.set_len(expected_size).await?;
        }

        let std_file = file.into_std().await;
        let mut mmap = tokio::task::spawn_blocking(move || -> Result<memmap2::MmapMut, std::io::Error> {
            unsafe { memmap2::MmapMut::map_mut(&std_file) }
        })
        .await
        .unwrap()?;

        let (mut hasher, mut mmap) = if existing_size > 0 {
            tokio::task::spawn_blocking(move || {
                let mut h = Md5::new();
                h.update(&mmap[..(existing_size as usize)]);
                (h, mmap)
            })
            .await
            .unwrap()
        } else {
            (Md5::new(), mmap)
        };

        if existing_size > 0 {
            on_progress(existing_size);
        }

        if existing_size < expected_size {
            let range_header = format!("bytes={}-", existing_size);
            let response = self
                .client
                .get(url)
                .header(RANGE, range_header)
                .send()
                .await?
                .error_for_status()?;
            
            let mut current_offset = existing_size as usize;

            if response.status() == reqwest::StatusCode::OK {
                hasher = Md5::new();
                current_offset = 0;
                on_progress(0);
            }

            let mut stream = response.bytes_stream();

            while let Some(chunk_result) = stream.next().await {
                let chunk = chunk_result?;
                hasher.update(&chunk);
                
                let len = chunk.len();
                mmap[current_offset..current_offset + len].copy_from_slice(&chunk);
                current_offset += len;
                
                on_progress(len as u64);
            }
        }

        tokio::task::spawn_blocking(move || mmap.flush()).await.unwrap()?;

        let final_md5 = hex::encode(hasher.finalize());
        if final_md5 != expected_md5 {
            let _ = fs::remove_file(tmp_path).await;
            return Err(Error::Md5Mismatch {
                expected: expected_md5.to_string(),
                actual: final_md5,
            });
        }

        Ok(())
    }
}

#[cfg(unix)]
async fn create_symlink(original: &Path, link: &Path) -> std::io::Result<()> {
    tokio::fs::symlink(original, link).await
}
#[cfg(windows)]
async fn create_symlink(original: &Path, link: &Path) -> std::io::Result<()> {
    tokio::fs::symlink_file(original, link).await
}
