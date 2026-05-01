use crate::error::Error;
use futures_util::StreamExt;
use md5::{Digest, Md5};
use reqwest::{header::RANGE, Client};
use std::path::{Path, PathBuf};
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};

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
        let bucket_filename = format!("{}.{}", expected_md5, expected_size);
        let bucket_path = self.bucket_dir.join(&bucket_filename);
        let tmp_path = self.bucket_dir.join(format!("tmp.{}", bucket_filename));

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // 1. 检查目标软链接
        if target_path.exists() || fs::symlink_metadata(target_path).await.is_ok() {
            if fs::read_link(target_path).await.map(|p| p == bucket_path).unwrap_or(false) && bucket_path.exists() {
                on_progress(expected_size); // 已同步
                return Ok(());
            } else {
                fs::remove_file(target_path).await?; // 失效链接，移除
            }
        }

        // 2. CAS 桶下载逻辑
        if !bucket_path.exists() {
            self.download_to_tmp(url, &tmp_path, expected_md5, expected_size, &mut on_progress).await?;
            fs::rename(&tmp_path, &bucket_path).await?; // 原子生效
        } else {
            on_progress(expected_size); // 桶里已有，直接满进度
        }

        // 3. 建立软链接
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
        let mut hasher = Md5::new();
        let mut file = OpenOptions::new().read(true).write(true).create(true).open(tmp_path).await?;

        let existing_size = file.metadata().await?.len();
        if existing_size > 0 {
            if existing_size > expected_size {
                file.set_len(0).await?;
            } else {
                let mut buf = [0u8; 65536];
                file.seek(SeekFrom::Start(0)).await?;
                loop {
                    let n = file.read(&mut buf).await?;
                    if n == 0 { break; }
                    hasher.update(&buf[..n]);
                }
                on_progress(existing_size);
            }
        }

        file.seek(SeekFrom::End(0)).await?;
        let existing_size = file.metadata().await?.len(); // 重新获取

        if existing_size < expected_size {
            let range_header = format!("bytes={}-", existing_size);
            let mut response = self.client.get(url).header(RANGE, range_header).send().await?.error_for_status()?;
            let mut stream = response.bytes_stream();

            while let Some(chunk_result) = stream.next().await {
                let chunk = chunk_result?;
                hasher.update(&chunk);
                file.write_all(&chunk).await?;
                on_progress(chunk.len() as u64);
            }
            file.flush().await?;
        }

        let final_md5 = hex::encode(hasher.finalize());
        if final_md5 != expected_md5 {
            drop(file);
            let _ = fs::remove_file(tmp_path).await;
            return Err(format!("MD5 mismatch! Expected {}, got {}", expected_md5, final_md5).into());
        }

        Ok(())
    }
}

#[cfg(unix)]
async fn create_symlink(original: &Path, link: &Path) -> std::io::Result<()> { tokio::fs::symlink(original, link).await }
#[cfg(windows)]
async fn create_symlink(original: &Path, link: &Path) -> std::io::Result<()> { tokio::fs::symlink_file(original, link).await }
