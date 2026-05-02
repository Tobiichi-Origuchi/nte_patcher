//! The central download management API.

use crate::config::PatcherConfig;
use crate::download::Downloader;
use crate::error::Error;
use crate::model::ResTask;
use futures::stream::{self, StreamExt};
use reqwest::Client;
use std::sync::Arc;

/// Coordinates the downloading of multiple tasks using a specified configuration.
pub struct DownloadManager {
    config: Arc<PatcherConfig>,
    downloader: Arc<Downloader>,
}

impl DownloadManager {
    /// Creates a new `DownloadManager` with the provided configuration.
    ///
    /// # Example
    /// ```rust
    /// use nte_patcher::config::PatcherConfig;
    /// use nte_patcher::manager::DownloadManager;
    ///
    /// let config = PatcherConfig::default();
    /// let manager = DownloadManager::new(config);
    /// ```
    pub fn new(config: PatcherConfig) -> Self {
        let client = Client::builder()
            .tcp_keepalive(std::time::Duration::from_secs(config.tcp_keepalive_secs))
            .build()
            .unwrap();

        let config = Arc::new(config);
        Self {
            config: config.clone(),
            downloader: Arc::new(Downloader::new(client, config)),
        }
    }

    fn build_url(&self, md5: &str, size: u64) -> String {
        let shard = md5.get(0..1).unwrap_or("0");
        format!("{}/Res/{}/{}.{}", self.config.base_url, shard, md5, size)
    }

    /// Starts all the provided tasks and tracks total progress via the callback.
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

        let mut stream =
            stream::iter(stream_iter).buffer_unordered(self.config.max_concurrent_tasks);

        while let Some(result) = stream.next().await {
            result?;
        }

        drop(stream);

        let _ = progress_task.await;

        Ok(())
    }
}
