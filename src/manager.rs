use crate::download::Downloader;
use crate::error::Error;
use crate::model::ResTask;
use futures::stream::{self, StreamExt};
use reqwest::Client;
use std::path::PathBuf;
use std::sync::Arc;

pub struct DownloadManager {
    base_url: String,
    downloader: Arc<Downloader>,
    max_concurrent_tasks: usize,
}

impl DownloadManager {
    pub fn new(
        base_url: &str,
        bucket_dir: PathBuf,
        game_dir: PathBuf,
        max_concurrent: usize,
    ) -> Self {
        let client = Client::builder()
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .build()
            .unwrap();

        Self {
            base_url: base_url.to_string(),
            downloader: Arc::new(Downloader::new(client, bucket_dir, game_dir)),
            max_concurrent_tasks: max_concurrent,
        }
    }

    fn build_url(&self, md5: &str, size: u64) -> String {
        let shard = md5.get(0..1).unwrap_or("0");
        format!("{}/Res/{}/{}.{}", self.base_url, shard, md5, size)
    }

    pub async fn start_all<F>(&self, tasks: Vec<ResTask>, mut on_progress: F) -> Result<(), Error>
    where
        F: FnMut(u64) + Send + 'static,
    {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<u64>();

        let progress_task = tokio::spawn(async move {
            while let Some(bytes) = rx.recv().await {
                on_progress(bytes);
            }
        });

        let stream_iter = tasks.into_iter().map({
            let base_tx = tx.clone();
            move |task| {
                let downloader = self.downloader.clone();
                let url = self.build_url(&task.md5, task.filesize);

                let task_tx = base_tx.clone();

                async move {
                    downloader
                        .execute_task(&url, &task, move |bytes| {
                            let _ = task_tx.send(bytes);
                        })
                        .await
                }
            }
        });

        drop(tx);

        let mut stream = stream::iter(stream_iter).buffer_unordered(self.max_concurrent_tasks);

        while let Some(result) = stream.next().await {
            result?;
        }

        let _ = progress_task.await;

        Ok(())
    }
}
