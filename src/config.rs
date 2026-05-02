use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PatcherConfig {
    pub base_url: String,
    pub bucket_dir: PathBuf,
    pub game_dir: PathBuf,
    pub max_concurrent_tasks: usize,
    pub retry_count: u32,
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
