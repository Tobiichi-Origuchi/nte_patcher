# NTE PatcherSDK Architecture Refactoring (Phase 2)

## Purpose
Address structural and organizational debt in the NTE PatcherSDK by decoupling components, standardizing error handling (adhering to domain-error patterns), and extracting hardcoded configuration values.

## Architecture & Design Changes

### 1. Configuration Extraction (`config.rs`)
- Create a `PatcherConfig` struct to serve as a single source of truth for runtime parameters.
- **Fields**:
  - `max_concurrent_tasks: usize`
  - `retry_count: u32`
  - `base_url: String`
  - `bucket_dir: PathBuf`
  - `game_dir: PathBuf`
- **Impact**: `DownloadManager` and `Downloader` will accept `Arc<PatcherConfig>` instead of individual fragmented parameters or hardcoded magic numbers (e.g., the hardcoded `3` for retries or `60s` for TCP keepalive).

### 2. Error Handling Optimization (`error.rs`)
- Refactor the flat `Error` enum into a structured, domain-driven design using `thiserror`.
- **New Structure**:
  - `Io(std::io::Error)`
  - `Network(reqwest::Error)`
  - `Checksum(expected, actual)`
  - `Configuration(String)`
  - `Validation(String)` - For parsing, zip, url, invalid payloads, etc.
- **Refining `is_retryable`**:
  - Transient errors (e.g., TCP timeouts, 5xx server errors, `Md5Mismatch`) return `true`.
  - Permanent errors (e.g., 404 Not Found, 401 Unauthorized, corrupted zip headers) return `false` to fail fast and prevent infinite loops.

### 3. Storage and Downloader Decoupling (`cas.rs` & `download.rs`)
- Currently, `cas::BucketManager` handles both the filesystem layout/symlinking and the actual HTTP downloading (`download_to_tmp`).
- **Storage Trait**:
  - Extract a `Storage` trait (or just a clean `BucketManager` struct with no `reqwest::Client`) that is solely responsible for local disk management (`get_bucket_path`, `get_tmp_path`, `commit_file`, `create_symlink`, `sync_file` (if it just coordinates local paths without downloading)).
  - Actually, `sync_file` currently contains download logic. The download loop will be moved entirely into `download.rs` (`Downloader`).
- **Downloader Refactor**:
  - `Downloader` handles network streams, ranges, and HTTP clients.
  - `cas.rs` handles paths, directories, and symlinks.
  - `Downloader` will call methods on `BucketManager` like `get_tmp_path` and `commit_file(tmp, bucket)`, separating the `HOW` to download from `WHERE` to store.

## Testing Strategy
- The refactor must not change the external behavior of the SDK. Existing integration tests in `test_integration.rs` will be used to ensure the public API behaves consistently.
- Future phases will find it easier to mock the `Storage` or inject isolated `PatcherConfig`s for unit tests.
