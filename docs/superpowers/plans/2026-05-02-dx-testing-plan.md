# DX and Testing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Improve DX via rustdocs, add comprehensive tests, and stabilize the CI pipeline.

**Architecture:** Use Rust's built-in `#[cfg(test)]` modules for unit tests. Use `cargo doc` to ensure `#![warn(missing_docs)]` passes. Modify GitHub Actions YAML for CI improvements.

**Tech Stack:** Rust, `rustdoc`, `tokio::test`, GitHub Actions

---

### Task 1: Rustdoc and Missing Docs

**Files:**
- Modify: `src/lib.rs` and all public modules (`config.rs`, `manager.rs`, `model.rs`, `error.rs`, etc.)

- [ ] **Step 1: Enable missing_docs lint**
In `src/lib.rs`:
```rust
#![warn(missing_docs)]
//! NTE Patcher SDK
//! 
//! A high-performance, concurrent downloading and patching SDK for game assets.
```

- [ ] **Step 2: Document `config.rs`**
Add `///` comments to `PatcherConfig` and its fields.

- [ ] **Step 3: Document `manager.rs`**
Add `///` comments to `DownloadManager`, `new`, and `start_all`. Provide a simple `/// ```rust` example in `new`.

- [ ] **Step 4: Document `model.rs` and `error.rs`**
Add basic `///` comments to all public enums/structs/fields.

- [ ] **Step 5: Run cargo doc**
Run: `cargo doc --no-deps`
Ensure no `missing_docs` warnings remain for public items. (You may need to add `#[allow(missing_docs)]` to deeply nested private-like public items if they are too tedious, but prefer adding brief docs).

- [ ] **Step 6: Commit**
```bash
git add src/
git commit -m "docs: add comprehensive rustdoc comments and missing_docs lint"
```

---

### Task 2: Unit Tests

**Files:**
- Modify: `src/crypto.rs`, `src/verify.rs`

- [ ] **Step 1: Add tests to `src/crypto.rs`**
At the bottom of `src/crypto.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_payload_length() {
        let key = [0u8; 16];
        let iv = [0u8; 16];
        let bad_data = vec![1, 2, 3]; // not multiple of 16
        
        let mut cursor = std::io::Cursor::new(bad_data);
        let res = decrypt_payload(&mut cursor, &key, &iv);
        assert!(matches!(res, Err(Error::Validation(_))));
    }
}
```

- [ ] **Step 2: Add tests to `src/verify.rs`**
At the bottom of `src/verify.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn test_empty_file_md5() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.txt");
        std::fs::File::create(&path).unwrap();
        
        // MD5 of empty string is d41d8cd98f00b204e9800998ecf8427e
        let res = check_file_md5(&path, "d41d8cd98f00b204e9800998ecf8427e").await.unwrap();
        assert!(res);
    }
}
```

- [ ] **Step 3: Run cargo test**
Run: `cargo test --lib`
Expected: PASS

- [ ] **Step 4: Commit**
```bash
git add src/crypto.rs src/verify.rs
git commit -m "test: add unit tests for crypto and verify modules"
```

---

### Task 3: Integration Tests

**Files:**
- Modify: `tests/test_integration.rs`

- [ ] **Step 1: Verify test_integration compiles and runs**
Check `tests/test_integration.rs` for any remaining outdated API usages from Phase 2. Ensure it compiles.
If the test depends on an external server that is timing out (as seen earlier), consider ignoring it or shortening the timeout. Since we don't control the external server, we can add `#[ignore]` to tests that require live external network if they are flaky, or mock the server. For this plan, just ensure it compiles. If the test is taking >60s, add a timeout or reduce the file size.

- [ ] **Step 2: Run cargo test --test test_integration**
Run: `cargo test --test test_integration`

- [ ] **Step 3: Commit**
```bash
git add tests/test_integration.rs
git commit -m "test: stabilize integration tests"
```

---

### Task 4: CI/CD Optimization

**Files:**
- Modify: `.github/workflows/rust.yml`

- [ ] **Step 1: Update rust.yml**
Ensure the `rust.yml` file includes caching and formatting checks:
```yaml
name: Rust

on:
  push:
    branches: [ "main" ]
  pull_request:
    branches: [ "main" ]

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    
    - name: Setup Rust
      uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt, clippy
        
    - name: Rust Cache
      uses: Swatinem/rust-cache@v2
      
    - name: Check formatting
      run: cargo fmt -- --check
      
    - name: Clippy
      run: cargo clippy --all-targets --all-features -- -D warnings
      
    - name: Run tests
      run: cargo test --verbose
```

- [ ] **Step 2: Commit**
```bash
git add .github/workflows/rust.yml
git commit -m "ci: optimize github actions with cache, fmt, and clippy"
```
