use crate::cas::BucketManager;
use crate::error::Error;
use crate::model::{ResTask, TaskType};
use crate::{retry, verify};
use futures_util::StreamExt;
use reqwest::{Client, header::RANGE};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};

pub struct Downloader {
    client: Client,
    cas_manager: Arc<BucketManager>,
    game_dir: PathBuf,
}

impl Downloader {
    pub fn new(client: Client, bucket_dir: PathBuf, game_dir: PathBuf) -> Self {
        Self {
            cas_manager: Arc::new(BucketManager::new(client.clone(), bucket_dir)),
            client,
            game_dir,
        }
    }

    pub async fn execute_task<F>(
        &self,
        url: &str,
        task: &ResTask,
        on_progress: F,
    ) -> Result<(), Error>
    where
        F: Fn(u64) + Send + Sync + Clone + 'static,
    {
        let url_c = url.to_string();
        let task_c = task.clone();
        let game_dir = self.game_dir.clone();

        retry::with_retry(3, || {
            let cas_mgr = self.cas_manager.clone();
            let client = self.client.clone();
            let url = url_c.clone();
            let task = task_c.clone();
            let game_dir = game_dir.clone();
            let prog = on_progress.clone();

            async move {
                match &task.task_type {
                    TaskType::Normal => {
                        let target_path = game_dir.join(&task.target_path);
                        cas_mgr
                            .sync_file(&url, &target_path, &task.md5, task.filesize, prog)
                            .await?;
                    }

                    TaskType::Pak { entries } => {
                        let pak_symlink_target =
                            game_dir.join(format!(".pak_cache/{}.pak", task.md5));
                        cas_mgr
                            .sync_file(
                                &url,
                                &pak_symlink_target,
                                &task.md5,
                                task.filesize,
                                prog.clone(),
                            )
                            .await?;

                        let pak_bucket_path = cas_mgr.get_bucket_path(&task.md5, task.filesize);
                        let mut pak_file = fs::File::open(&pak_bucket_path).await?;

                        for entry in entries {
                            let entry_target = game_dir.join(&entry.name);
                            let entry_bucket_path = cas_mgr.get_bucket_path(&entry.md5, entry.size);

                            if let Some(parent) = entry_target.parent() {
                                fs::create_dir_all(parent).await?;
                            }

                            if !entry_bucket_path.exists() {
                                if let Some(parent) = entry_bucket_path.parent() {
                                    fs::create_dir_all(parent).await?;
                                }
                                let tmp_path = cas_mgr.get_tmp_path(&entry.md5, entry.size);
                                let mut out_file = fs::File::create(&tmp_path).await?;

                                pak_file.seek(SeekFrom::Start(entry.offset)).await?;
                                let mut chunk = (&mut pak_file).take(entry.size);
                                tokio::io::copy(&mut chunk, &mut out_file).await?;
                                out_file.flush().await?;

                                fs::rename(&tmp_path, &entry_bucket_path).await?;
                            }

                            let _ = fs::remove_file(&entry_target).await;
                            #[cfg(unix)]
                            fs::symlink(&entry_bucket_path, &entry_target).await?;
                            #[cfg(windows)]
                            fs::symlink_file(&entry_bucket_path, &entry_target).await?;
                        }

                        let _ = fs::remove_file(&pak_symlink_target).await;
                    }

                    TaskType::Block { blocks } => {
                        let target_path = game_dir.join(&task.target_path);
                        let bucket_path = cas_mgr.get_bucket_path(&task.md5, task.filesize);
                        let tmp_path = cas_mgr.get_tmp_path(&task.md5, task.filesize);

                        if bucket_path.exists() {
                            prog(task.filesize);
                        } else {
                            if let Some(parent) = target_path.parent() {
                                fs::create_dir_all(parent).await?;
                            }
                            if let Some(parent) = bucket_path.parent() {
                                fs::create_dir_all(parent).await?;
                            }

                            let mut file = fs::OpenOptions::new()
                                .write(true)
                                .read(true)
                                .create(true)
                                .truncate(false)
                                .open(&tmp_path)
                                .await?;
                            if file.metadata().await?.len() != task.filesize {
                                file.set_len(task.filesize).await?;
                            }

                            for block in blocks {
                                let range_header = format!(
                                    "bytes={}-{}",
                                    block.start,
                                    block.start + block.size - 1
                                );

                                let response = client
                                    .get(&url)
                                    .header(RANGE, range_header)
                                    .send()
                                    .await?
                                    .error_for_status()?;

                                file.seek(SeekFrom::Start(block.start)).await?;
                                let mut stream = response.bytes_stream();

                                while let Some(chunk) = stream.next().await {
                                    let data = chunk?;
                                    file.write_all(&data).await?;
                                    prog(data.len() as u64);
                                }
                            }
                            file.flush().await?;
                            drop(file);

                            if !verify::check_file_md5(&tmp_path, &task.md5).await? {
                                let _ = fs::remove_file(&tmp_path).await;
                                return Err(Error::Md5Mismatch {
                                    expected: task.md5.clone(),
                                    actual: String::from("unknown (block validation)"),
                                });
                            }

                            fs::rename(&tmp_path, &bucket_path).await?;
                        }

                        let _ = fs::remove_file(&target_path).await;
                        #[cfg(unix)]
                        fs::symlink(&bucket_path, &target_path).await?;
                        #[cfg(windows)]
                        fs::symlink_file(&bucket_path, &target_path).await?;
                    }
                }
                Ok(())
            }
        })
        .await
    }
}
