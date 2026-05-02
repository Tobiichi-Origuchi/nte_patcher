# NTE PatcherSDK Performance Optimization (Phase 1)

## Purpose
Optimize file I/O and MD5 hashing using aggressive memory mapping (`memmap2`), eliminating the overhead of `tokio::task::spawn_blocking` associated with `tokio::fs::write_all` on many small chunks.

## Architecture & Data Flow

1. **Hashing (`verify.rs` & `cas.rs`)**
   - Replace standard `File::read` chunk loops with zero-copy `memmap2::Mmap`.
   - The OS virtual memory manager will handle paging, making MD5 hashing purely CPU-bound without manual buffering.

2. **Sequential Download (`cas.rs` -> `sync_file`)**
   - Allocate the `tmp_path` to `expected_size`.
   - Map it as `MmapMut`.
   - As chunks arrive from the network stream, copy them into the mapped slice: `mmap[offset..offset+len].copy_from_slice(&chunk)`.

3. **Concurrent Block Download (`download.rs` -> `TaskType::Block`)**
   - Pre-allocate the full file.
   - Create a single `memmap2::MmapMut` and wrap it in a custom `SyncMmap` struct.
   - **`SyncMmap`**: Holds a raw pointer `*mut u8` to the mapped memory and the total length. Implements `Send + Sync`.
   - Provides a safe method: `fn write_at(&self, offset: usize, data: &[u8])`.
   - **SAFETY**: Since `TaskType::Block` assigns distinct non-overlapping byte ranges to different concurrent tasks, writing via raw pointer is data-race free. The `write_at` method will include strict bounds checking to prevent buffer overflow.

4. **Progress Tracking (`download.rs`)**
   - Keep the existing `AtomicU64` mechanism, as it accurately handles retries.

## Error Handling
- Memory mapping requires the file to be opened with correct permissions.
- If a server sends more data than expected (exceeding block size), `SyncMmap::write_at` will return an error or panic before writing, ensuring memory safety.

## Dependencies
- Add `memmap2 = "0.9"` to `Cargo.toml`.

## Testing
- Run existing integration tests to ensure downloads and hashing produce correct results.
