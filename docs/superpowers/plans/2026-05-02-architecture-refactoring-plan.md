# Architecture Refactoring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract configuration, optimize error handling with `thiserror`, and decouple the storage manager from the network downloader.

**Architecture:** 
1. Introduce `PatcherConfig` to hold parameters.
2. Refactor `Error` into structured domain variants.
3. Move `reqwest::Client` out of `BucketManager`. Move `sync_file` and `download_to_tmp` into `Downloader`. `BucketManager` becomes a pure local storage utility.

**Tech Stack:** Rust, `tokio`, `reqwest`, `thiserror`

---

### Task 1: Configuration Extraction (`config.rs`)

**Files:**
- Create/Modify: `src/config.rs`
- Modify: `src/manager.rs`, `src/download.rs`

- [ ] **Step 1: Define PatcherConfig in config.rs**

```rust
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
```

- [ ] **Step 2: Update DownloadManager to use PatcherConfig**

Modify `src/manager.rs`:
```rust
use crate::config::PatcherConfig;
use crate::download::Downloader;
use crate::error::Error;
use crate::model::ResTask;
use futures::stream::{self, StreamExt};
use reqwest::Client;
use std::sync::Arc;

pub struct DownloadManager {
    config: Arc<PatcherConfig>,
    downloader: Arc<Downloader>,
}

impl DownloadManager {
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
        let mut stream = stream::iter(stream_iter).buffer_unordered(self.config.max_concurrent_tasks);
        while let Some(result) = stream.next().await {
            result?;
        }
        drop(stream);
        let _ = progress_task.await;
        Ok(())
    }
}
```

- [ ] **Step 3: Update Downloader to use PatcherConfig**

Modify `src/download.rs` (just the struct and `new` method for now):
```rust
use crate::config::PatcherConfig;
// ... other imports

pub struct Downloader {
    client: Client,
    cas_manager: Arc<BucketManager>,
    config: Arc<PatcherConfig>,
}

impl Downloader {
    pub fn new(client: Client, config: Arc<PatcherConfig>) -> Self {
        Self {
            cas_manager: Arc::new(BucketManager::new(client.clone(), config.bucket_dir.clone())),
            client,
            config,
        }
    }
// ...
```

- [ ] **Step 4: Fix Downloader execute_task to use config.retry_count**

In `src/download.rs` inside `execute_task`:
Change `retry::with_retry(3, || { ...` to `retry::with_retry(self.config.retry_count, || { ...`

- [ ] **Step 5: Run tests/check**

Run: `cargo check`
(Expected: some warnings, but it should compile. If `config.rs` wasn't exported in `lib.rs`, add it).

Modify `src/lib.rs` if needed: `pub mod config;`

- [ ] **Step 6: Commit**

```bash
git add src/config.rs src/lib.rs src/manager.rs src/download.rs
git commit -m "feat: extract PatcherConfig and inject via Arc"
```

---

### Task 2: Error Handling Refactoring (`error.rs`)

**Files:**
- Modify: `src/error.rs`

- [ ] **Step 1: Rewrite Error Enum**

```rust
use reqwest::StatusCode;
use std::io::ErrorKind;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Checksum mismatch! Expected {expected}, got {actual}")]
    Checksum { expected: String, actual: String },
    
    #[error("Validation error: {0}")]
    Validation(String),
}

impl From<quick_xml::DeError> for Error {
    fn from(e: quick_xml::DeError) -> Self {
        Self::Validation(format!("XML error: {}", e))
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(e: zip::result::ZipError) -> Self {
        Self::Validation(format!("Zip error: {}", e))
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Self::Validation(format!("URL parsing error: {}", e))
    }
}

impl Error {
    pub fn is_retryable(&self) -> bool {
        match self {
            Error::Network(e) => {
                if let Some(status) = e.status() {
                    status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS || status == StatusCode::REQUEST_TIMEOUT
                } else {
                    true
                }
            }
            Error::Checksum { .. } => true,
            Error::Io(e) => matches!(
                e.kind(),
                ErrorKind::ConnectionAborted
                    | ErrorKind::ConnectionReset
                    | ErrorKind::ConnectionRefused
                    | ErrorKind::TimedOut
                    | ErrorKind::Interrupted
                    | ErrorKind::UnexpectedEof
            ),
            Error::Validation(_) => false,
        }
    }
}
```

- [ ] **Step 2: Update usages of old Error variants**

Run: `cargo check`
You will see errors where `Error::Md5Mismatch` or `Error::InvalidPayload` were used.
Replace `Error::Md5Mismatch` with `Error::Checksum`.
Replace `Error::InvalidPayload` and `Error::InvalidPadding` with `Error::Validation("Invalid...".to_string())`.

For example in `src/cas.rs`:
```rust
return Err(Error::Checksum {
    expected: expected_md5.to_string(),
    actual: final_md5,
});
```

And in `src/download.rs`:
```rust
return Err(Error::Checksum {
    expected: task.md5.clone(),
    actual: String::from("unknown (block validation)"),
});
```

- [ ] **Step 3: Run check**

Run: `cargo check`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/error.rs src/cas.rs src/download.rs src/crypto.rs
git commit -m "refactor: structured domain error handling with thiserror"
```

---

### Task 3: Decouple Storage and Downloader (`cas.rs` & `download.rs`)

**Files:**
- Modify: `src/cas.rs`, `src/download.rs`

- [ ] **Step 1: Remove reqwest::Client from BucketManager**

In `src/cas.rs`:
```rust
use std::path::{Path, PathBuf};
use tokio::fs;

pub struct BucketManager {
    pub bucket_dir: PathBuf,
}

impl BucketManager {
    pub fn new(bucket_dir: impl AsRef<Path>) -> Self {
        Self {
            bucket_dir: bucket_dir.as_ref().to_path_buf(),
        }
    }
    // keep get_bucket_path, get_tmp_path
// ...
```
Remove `client: Client` from `BucketManager`.
Remove `pub async fn sync_file` and `async fn download_to_tmp` from `BucketManager` (delete them from `src/cas.rs`).
Keep `create_symlink` helper, make it `pub async fn create_symlink`.

- [ ] **Step 2: Move sync_file and download_to_tmp to Downloader**

In `src/download.rs`, update `Downloader::new`:
```rust
    pub fn new(client: Client, config: Arc<PatcherConfig>) -> Self {
        Self {
            cas_manager: Arc::new(BucketManager::new(config.bucket_dir.clone())),
            client,
            config,
        }
    }
```

Paste `sync_file` and `download_to_tmp` inside `impl Downloader`.

Update `sync_file` signature and body:
```rust
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
        let bucket_path = self.cas_manager.get_bucket_path(expected_md5, expected_size);
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
```

Update `download_to_tmp` signature:
```rust
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
       // ... existing code from cas.rs but using `self.client.get(url)`
```
Remember to add `use reqwest::header::RANGE;`, `use md5::{Digest, Md5};`, `use tokio::fs::OpenOptions;` at the top of `src/download.rs`.

- [ ] **Step 3: Fix Downloader execute_task to use `self.sync_file` instead of `cas_mgr.sync_file`**

In `execute_task`:
```rust
        retry::with_retry(self.config.retry_count, || {
            let client = self.client.clone();
            let url = url_c.clone();
            let task = task_c.clone();
            // Since we need to call self.sync_file, we can't easily move `self` into the closure if it requires 'static. Wait, `execute_task` takes `&self`.
```
Wait, `retry::with_retry` closure requires `Fn() -> impl Future`. Since `Downloader` has no lifetime, we can clone `self` if we wrap Downloader's inner state, or we can just pass an `Arc<Self>` or copy references.
Actually, it's easier: just `let cas_mgr = self.cas_manager.clone();` and we can copy the code of `sync_file` into a standalone async function, OR make `Downloader` cheaply cloneable by wrapping its fields in `Arc` (it already has `client` which is cheap to clone, `cas_manager` which is `Arc`, and `config` which is `Arc`). So we can add `#[derive(Clone)]` to `Downloader`.

Modify `Downloader` struct:
```rust
#[derive(Clone)]
pub struct Downloader {
    client: Client,
    cas_manager: Arc<BucketManager>,
    config: Arc<PatcherConfig>,
}
```

Then inside `execute_task`:
```rust
        let this = self.clone();
        retry::with_retry(self.config.retry_count, || {
            let this = this.clone();
            let cas_mgr = this.cas_manager.clone();
            let client = this.client.clone();
            let url = url_c.clone();
            // ...
            async move {
                match &task.task_type {
                    TaskType::Normal => {
                        let target_path = game_dir.join(&task.target_path);
                        this.sync_file(&url, &target_path, &task.md5, task.filesize, prog).await?;
                    }
                    TaskType::Pak { entries } => {
                        let pak_symlink_target = game_dir.join(format!(".pak_cache/{}.pak", task.md5));
                        this.sync_file(&url, &pak_symlink_target, &task.md5, task.filesize, prog.clone()).await?;
```
And make sure `create_symlink` is called using `crate::cas::create_symlink`.

- [ ] **Step 4: Check compilation and tests**

Run: `cargo test`
Expected: Tests should pass as behavior is unchanged.

- [ ] **Step 5: Commit**

```bash
git add src/cas.rs src/download.rs
git commit -m "refactor: decouple BucketManager from networking logic"
```
