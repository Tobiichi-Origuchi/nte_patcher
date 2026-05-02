# NTE PatcherSDK

A high-performance, concurrent NTE game asset downloading and patching SDK implemented in Rust.

## Features

- **Extreme Performance**: Leverages aggressive memory mapping (`memmap2`) for zero-copy file hashing (MD5) and concurrent direct-to-disk writing.
- **Efficient Concurrency**: Managed asynchronous downloads using `tokio` and `reqwest`, supporting both sequential and block-based parallel downloads.
- **Robust Storage Architecture**: Cleanly decoupled Storage and Network layers for reliable asset management and symlinking.
- **Type-Safe Configuration**: Centralized `PatcherConfig` for easy management of concurrency limits, retry policies, and timeouts.
- **Domain-Driven Error Handling**: Structured error reporting using `thiserror`, with intelligent retry logic for transient network or I/O failures.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
nte_patcher = 0.2
```

### Quick Start

```rust
use nte_patcher::config::PatcherConfig;
use nte_patcher::manager::DownloadManager;
use nte_patcher::model::ResTask;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Define your configuration
    let config = PatcherConfig {
        base_url: "https://your-cdn.com/assets".to_string(),
        bucket_dir: PathBuf::from("./cache/bucket"),
        game_dir: PathBuf::from("./game"),
        max_concurrent_tasks: 8,
        retry_count: 3,
        tcp_keepalive_secs: 60,
    };

    // 2. Initialize the manager
    let manager = DownloadManager::new(config);

    // 3. Define your tasks (usually parsed from ResList.xml)
    let tasks: Vec<ResTask> = Vec::new(); // Populate with your actual tasks

    // 4. Start downloading with progress tracking
    manager.start_all(tasks, |delta_bytes| {
        println!("Downloaded {} bytes", delta_bytes);
    }).await?;

    Ok(())
}
```

## Architecture

The SDK is split into several core modules:
- **`manager`**: High-level API for coordinating download batches.
- **`download`**: Handles HTTP streaming, range requests, and retry logic.
- **`cas`**: Content Addressable Storage (Bucket) management and filesystem symlinking.
- **`verify`**: High-speed zero-copy MD5 checksum verification.
- **`crypto`**: AES-CBC decryption for resource list files.

## License

This project is licensed under the MIT License.
