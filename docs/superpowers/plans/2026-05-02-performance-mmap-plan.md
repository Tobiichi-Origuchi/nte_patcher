# Performance Mmap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement zero-copy MD5 hashing and aggressive memory-mapped file I/O to improve PatcherSDK performance.

**Architecture:** Use `memmap2` for zero-copy file reading in `verify.rs`, and a custom thread-safe `SyncMmap` wrapper to allow concurrent asynchronous network streams to write directly to distinct regions of a pre-allocated file without blocking threads.

**Tech Stack:** Rust, `tokio`, `memmap2`, `md-5`

---

### Task 1: Add Dependencies and SyncMmap Utility

**Files:**
- Create: `src/mmap.rs`
- Modify: `Cargo.toml`, `src/lib.rs`

- [ ] **Step 1: Update Cargo.toml to include memmap2**

```toml
[dependencies]
# ... (existing deps)
memmap2 = "0.9.3"
```

- [ ] **Step 2: Add mod to lib.rs**

```rust
// In src/lib.rs, add:
pub mod mmap;
```

- [ ] **Step 3: Implement SyncMmap in `src/mmap.rs`**

```rust
use memmap2::MmapMut;
use std::io::{Error, ErrorKind, Result};

pub struct SyncMmap {
    ptr: *mut u8,
    len: usize,
    _mmap: MmapMut,
}

unsafe impl Send for SyncMmap {}
unsafe impl Sync for SyncMmap {}

impl SyncMmap {
    pub fn new(mut mmap: MmapMut) -> Self {
        let ptr = mmap.as_mut_ptr();
        let len = mmap.len();
        Self { ptr, len, _mmap: mmap }
    }

    pub fn write_at(&self, offset: usize, data: &[u8]) -> Result<()> {
        if offset + data.len() > self.len {
            return Err(Error::new(ErrorKind::InvalidInput, "Buffer overflow"));
        }
        // SAFETY: Bounds checked. Caller guarantees concurrent writes are non-overlapping.
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), self.ptr.add(offset), data.len());
        }
        Ok(())
    }
}
```

- [ ] **Step 4: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/lib.rs src/mmap.rs
git commit -m "feat: add memmap2 dependency and SyncMmap utility"
```

---

### Task 2: Refactor MD5 Verification (`src/verify.rs`)

**Files:**
- Modify: `src/verify.rs`

- [ ] **Step 1: Update `check_file_md5` to use Mmap**

```rust
// Replace file reading loop with:
let is_match = tokio::task::spawn_blocking(move || -> Result<bool, std::io::Error> {
    let file = std::fs::File::open(&path_buf)?;
    // Handle empty file case
    if file.metadata()?.len() == 0 {
        let empty_md5 = Md5::digest(&[]);
        return Ok(empty_md5.as_slice() == expected_bytes);
    }
    
    // SAFETY: File might be modified externally, but this is a patcher SDK.
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    let hash = Md5::digest(&mmap);
    Ok(hash.as_slice() == expected_bytes)
})
```

- [ ] **Step 2: Update `check_slice_md5` to use Mmap**

```rust
// Replace file seek/read loop with:
let is_match = tokio::task::spawn_blocking(move || -> Result<bool, std::io::Error> {
    let file = std::fs::File::open(&path_buf)?;
    if file.metadata()?.len() < start + size {
        return Ok(false);
    }
    
    if size == 0 {
        let empty_md5 = Md5::digest(&[]);
        return Ok(empty_md5.as_slice() == expected_bytes);
    }

    // Map only the specific region
    // SAFETY: File might be modified externally, but we control the game dir.
    let mmap = unsafe { 
        memmap2::MmapOptions::new().offset(start).len(size as usize).map(&file)? 
    };
    
    let hash = Md5::digest(&mmap);
    Ok(hash.as_slice() == expected_bytes)
})
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/verify.rs
git commit -m "perf: use memmap2 for zero-copy md5 verification"
```

---

### Task 3: Refactor Sequential Download (`src/cas.rs`)

**Files:**
- Modify: `src/cas.rs`

- [ ] **Step 1: Pre-allocate and map file in `sync_file`**

```rust
// Around line 125, when existing_size < expected_size:
// Ensure file is at least expected_size
if file.metadata().await?.len() < expected_size {
    file.set_len(expected_size).await?;
}

// Convert tokio File to std File for mmap
let std_file = file.into_std().await;
let mut mmap = tokio::task::spawn_blocking(move || -> Result<memmap2::MmapMut, std::io::Error> {
    unsafe { memmap2::MmapMut::map_mut(&std_file) }
}).await.unwrap()?;

// In the stream loop:
let mut current_offset = existing_size as usize;
while let Some(chunk_result) = stream.next().await {
    let chunk = chunk_result?;
    hasher.update(&chunk);
    
    let len = chunk.len();
    mmap[current_offset..current_offset + len].copy_from_slice(&chunk);
    current_offset += len;
    
    on_progress(len as u64);
}
tokio::task::spawn_blocking(move || mmap.flush()).await.unwrap()?;
```

- [ ] **Step 2: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/cas.rs
git commit -m "perf: use memmap2 for sequential sync_file writing"
```

---

### Task 4: Refactor Concurrent Block Download (`src/download.rs`)

**Files:**
- Modify: `src/download.rs`

- [ ] **Step 1: Setup SyncMmap in `execute_task`**

```rust
// Add to top of file:
use crate::mmap::SyncMmap;

// Under TaskType::Block around line 170:
// file.set_len(task.filesize).await?;
// ...
let std_file = std::fs::OpenOptions::new()
    .read(true)
    .write(true)
    .open(&tmp_path)?;
let mmap = unsafe { memmap2::MmapMut::map_mut(&std_file)? };
let sync_mmap = std::sync::Arc::new(SyncMmap::new(mmap));
```

- [ ] **Step 2: Update block download stream**

```rust
// In the blocks stream map closure:
let sync_mmap = sync_mmap.clone();
async move {
    // ...
    let mut current_offset = block.start as usize;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let data = chunk?;
        sync_mmap.write_at(current_offset, &data)?;
        current_offset += data.len();
        prog(data.len() as u64);
    }
    // file.flush().await?; is no longer needed per block
    Ok::<(), Error>(())
}
```

- [ ] **Step 3: Run integration tests**

Run: `cargo test`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/download.rs
git commit -m "perf: use SyncMmap for concurrent block downloading"
```
