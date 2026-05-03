//! The central download management API.

use crate::config::PatcherConfig;
use crate::download::Downloader;
use crate::error::Error;
use crate::model::ResTask;
use futures_util::stream::{self, StreamExt};
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

    /// Starts all the provided tasks.
    ///
    /// `task_handler` is invoked just before each task is polled (subject to `max_concurrent_tasks`).
    /// It should return a tuple containing two closures:
    /// 1. A progress closure `P` called during the download with the number of bytes downloaded.
    /// 2. A finish closure `F` called when the task completes.
    pub async fn start_all<T, P, F>(
        &self,
        tasks: Vec<ResTask>,
        task_handler: T,
    ) -> Result<(), Error>
    where
        T: Fn(&ResTask) -> (P, F) + Send + Sync + 'static,
        P: Fn(u64) + Send + Sync + Clone + 'static,
        F: FnOnce() + Send + 'static,
    {
        let task_handler = Arc::new(task_handler);
        let stream_iter = tasks.into_iter().map(|task| {
            let downloader = self.downloader.clone();
            let url = self.build_url(&task.md5, task.filesize);
            let handler = task_handler.clone();

            async move {
                let (progress_cb, finish_cb) = handler(&task);
                let res = downloader.execute_task(&url, &task, progress_cb).await;
                finish_cb();
                res
            }
        });

        let mut stream =
            stream::iter(stream_iter).buffer_unordered(self.config.max_concurrent_tasks);

        while let Some(result) = stream.next().await {
            result?;
        }

        Ok(())
    }
}
