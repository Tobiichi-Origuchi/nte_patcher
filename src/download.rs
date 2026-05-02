#![allow(missing_docs)]
use crate::cas::BucketManager;
use crate::config::PatcherConfig;
use crate::error::Error;
use crate::model::{ResTask, TaskType};
use crate::{retry, verify};
use futures_util::StreamExt;
use reqwest::{Client, header::RANGE};
use std::sync::Arc;
use tokio::fs;

#[derive(Clone)]
pub struct Downloader {
    client: Client,
    cas_manager: Arc<BucketManager>,
    config: Arc<PatcherConfig>,
}

impl Downloader {
    pub fn new(client: Client, config: Arc<PatcherConfig>) -> Self {
        Self {
            cas_manager: Arc::new(BucketManager::new(config.bucket_dir.clone())),
            client,
            config,
        }
    }

    pub async fn sync_file<F>(
        &self,
        url: &str,
        target_path: &std::path::Path,
        expected_md5: &str,
        expected_size: u64,
        mut on_progress: F,
    ) -> Result<(), Error>
    where
        F: FnMut(u64),
    {
        let bucket_path = self
            .cas_manager
            .get_bucket_path(expected_md5, expected_size);
        let tmp_path = self.cas_manager.get_tmp_path(expected_md5, expected_size);

        if let Some(parent) = bucket_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        if target_path.exists() || fs::symlink_metadata(target_path).await.is_ok() {
            if let Ok(p) = fs::read_link(target_path).await {
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

        crate::cas::create_symlink(&bucket_path, target_path).await?;
        Ok(())
    }

    async fn download_to_tmp<F>(
        &self,
        url: &str,
        tmp_path: &std::path::Path,
        expected_md5: &str,
        expected_size: u64,
        on_progress: &mut F,
    ) -> Result<(), Error>
    where
        F: FnMut(u64),
    {
        use md5::{Digest, Md5};
        use tokio::fs::OpenOptions;

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
        let mmap =
            tokio::task::spawn_blocking(move || -> Result<memmap2::MmapMut, std::io::Error> {
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

        tokio::task::spawn_blocking(move || mmap.flush())
            .await
            .unwrap()?;

        let final_md5 = hex::encode(hasher.finalize());
        if final_md5 != expected_md5 {
            let _ = fs::remove_file(tmp_path).await;
            return Err(Error::Checksum {
                expected: expected_md5.to_string(),
                actual: final_md5,
            });
        }

        Ok(())
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
        let game_dir = self.config.game_dir.clone();

        let highest_reported = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let this = self.clone();

        retry::with_retry(self.config.retry_count, || {
            let this = this.clone();
            let cas_mgr = this.cas_manager.clone();
            let client = this.client.clone();
            let url = url_c.clone();
            let task = task_c.clone();
            let game_dir = game_dir.clone();

            let try_progress = Arc::new(std::sync::atomic::AtomicU64::new(0));
            let highest = highest_reported.clone();
            let original_prog = on_progress.clone();

            let prog = move |delta: u64| {
                if delta == 0 {
                    return;
                }
                let new_current =
                    try_progress.fetch_add(delta, std::sync::atomic::Ordering::Relaxed) + delta;
                let mut old_highest = highest.load(std::sync::atomic::Ordering::Acquire);
                loop {
                    if new_current <= old_highest {
                        break;
                    }
                    match highest.compare_exchange_weak(
                        old_highest,
                        new_current,
                        std::sync::atomic::Ordering::SeqCst,
                        std::sync::atomic::Ordering::Acquire,
                    ) {
                        Ok(_) => {
                            original_prog(new_current - old_highest);
                            break;
                        }
                        Err(h) => old_highest = h,
                    }
                }
            };

            async move {
                match &task.task_type {
                    TaskType::Normal => {
                        let target_path = game_dir.join(&task.target_path);
                        this.sync_file(&url, &target_path, &task.md5, task.filesize, prog)
                            .await?;
                    }

                    TaskType::Pak { entries } => {
                        let pak_symlink_target =
                            game_dir.join(format!(".pak_cache/{}.pak", task.md5));
                        this.sync_file(
                            &url,
                            &pak_symlink_target,
                            &task.md5,
                            task.filesize,
                            prog.clone(),
                        )
                        .await?;

                        let pak_bucket_path = cas_mgr.get_bucket_path(&task.md5, task.filesize);

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
                                let pak_path_c = pak_bucket_path.clone();
                                let tmp_path_c = tmp_path.clone();
                                let offset = entry.offset;
                                let size = entry.size;

                                tokio::task::spawn_blocking(move || -> std::io::Result<()> {
                                    use std::io::{Read, Seek, SeekFrom, Write};
                                    let mut p_file = std::fs::File::open(&pak_path_c)?;
                                    p_file.seek(SeekFrom::Start(offset))?;
                                    let mut chunk = p_file.take(size);

                                    let t_file = std::fs::File::create(&tmp_path_c)?;
                                    let mut buf_writer =
                                        std::io::BufWriter::with_capacity(65536, t_file);

                                    std::io::copy(&mut chunk, &mut buf_writer)?;
                                    buf_writer.flush()?;
                                    Ok(())
                                })
                                .await
                                .unwrap()?;

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

                            let file = fs::OpenOptions::new()
                                .write(true)
                                .read(true)
                                .create(true)
                                .truncate(false)
                                .open(&tmp_path)
                                .await?;
                            if file.metadata().await?.len() != task.filesize {
                                file.set_len(task.filesize).await?;
                            }
                            let std_file = file.into_std().await;
                            let mmap = tokio::task::spawn_blocking(
                                move || -> Result<memmap2::MmapMut, std::io::Error> {
                                    unsafe { memmap2::MmapMut::map_mut(&std_file) }
                                },
                            )
                            .await
                            .unwrap()?;
                            let sync_mmap = std::sync::Arc::new(crate::mmap::SyncMmap::new(mmap));

                            let mut stream = futures_util::stream::iter(
                                blocks.clone().into_iter().map(|block| {
                                    let tmp_path = tmp_path.clone();
                                    let client = client.clone();
                                    let url = url.clone();
                                    let prog = prog.clone();
                                    let block = block.clone();
                                    let sync_mmap = sync_mmap.clone();

                                    async move {
                                        if verify::check_slice_md5(
                                            &tmp_path,
                                            block.start,
                                            block.size,
                                            &block.md5,
                                        )
                                        .await
                                        .unwrap_or(false)
                                        {
                                            prog(block.size);
                                            return Ok::<(), Error>(());
                                        }

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

                                        let mut current_offset = block.start as usize;
                                        let mut stream = response.bytes_stream();

                                        while let Some(chunk) = stream.next().await {
                                            let data = chunk?;
                                            sync_mmap.write_at(current_offset, &data)?;
                                            current_offset += data.len();
                                            prog(data.len() as u64);
                                        }
                                        Ok::<(), Error>(())
                                    }
                                }),
                            )
                            .buffer_unordered(8);

                            while let Some(result) = stream.next().await {
                                result?;
                            }

                            if !verify::check_file_md5(&tmp_path, &task.md5).await? {
                                let _ = fs::remove_file(&tmp_path).await;
                                return Err(Error::Checksum {
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
