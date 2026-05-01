use crate::error::Error;
use futures_util::StreamExt;
use md5::{Digest, Md5};
use reqwest::{Client, header::RANGE};
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
        let mut hasher = Md5::new();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(tmp_path)
            .await?;

        let existing_size = file.metadata().await?.len();
        if existing_size > 0 {
            if existing_size > expected_size {
                file.set_len(0).await?;
            } else {
                let mut buf = [0u8; 65536];
                file.seek(SeekFrom::Start(0)).await?;
                loop {
                    let n = file.read(&mut buf).await?;
                    if n == 0 {
                        break;
                    }
                    hasher.update(&buf[..n]);
                }
                on_progress(existing_size);
            }
        }

        file.seek(SeekFrom::End(0)).await?;
        let existing_size = file.metadata().await?.len();

        if existing_size < expected_size {
            let range_header = format!("bytes={}-", existing_size);
            let response = self
                .client
                .get(url)
                .header(RANGE, range_header)
                .send()
                .await?
                .error_for_status()?;
            // if the server responds with a 200 OK, reset the file and start over
            if response.status() == reqwest::StatusCode::OK {
                file.set_len(0).await?;
                file.seek(SeekFrom::Start(0)).await?;
                hasher = Md5::new();
                on_progress(0);
            }
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
