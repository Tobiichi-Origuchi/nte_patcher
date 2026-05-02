//! Configuration definitions for NTE PatcherSDK.

use std::path::PathBuf;

/// Configuration for the download manager and downloader.
#[derive(Debug, Clone)]
pub struct PatcherConfig {
    /// The base URL for fetching resources.
    pub base_url: String,
    /// The directory where raw, downloaded blocks/files are cached.
    pub bucket_dir: PathBuf,
    /// The target directory where the final game files are assembled/symlinked.
    pub game_dir: PathBuf,
    /// The maximum number of concurrent download tasks allowed.
    pub max_concurrent_tasks: usize,
    /// The number of times to retry a failed, retryable download task.
    pub retry_count: u32,
    /// The TCP keepalive duration in seconds for the HTTP client.
    pub tcp_keepalive_secs: u64,
}

impl Default for PatcherConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            bucket_dir: PathBuf::new(),
            game_dir: PathBuf::new(),
            max_concurrent_tasks: 8,
            retry_count: 3,
            tcp_keepalive_secs: 60,
        }
    }
}
