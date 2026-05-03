#![allow(missing_docs)]
#[cfg(feature = "mmap")]
use memmap2::MmapMut;
use std::io::{Error, ErrorKind, Result};

#[cfg(feature = "mmap")]
pub struct SyncMmap {
    ptr: *mut u8,
    len: usize,
    _mmap: MmapMut,
}

#[cfg(feature = "mmap")]
unsafe impl Send for SyncMmap {}
#[cfg(feature = "mmap")]
unsafe impl Sync for SyncMmap {}

#[cfg(feature = "mmap")]
impl SyncMmap {
    pub fn new(mut mmap: MmapMut) -> Self {
        let ptr = mmap.as_mut_ptr();
        let len = mmap.len();
        Self {
            ptr,
            len,
            _mmap: mmap,
        }
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

#[cfg(not(feature = "mmap"))]
pub struct SyncMmap {
    file: std::sync::Mutex<std::fs::File>,
    len: usize,
}

#[cfg(not(feature = "mmap"))]
impl SyncMmap {
    pub fn new(file: std::fs::File, len: usize) -> Self {
        Self {
            file: std::sync::Mutex::new(file),
            len,
        }
    }

    pub fn write_at(&self, offset: usize, data: &[u8]) -> Result<()> {
        if offset + data.len() > self.len {
            return Err(Error::new(ErrorKind::InvalidInput, "Buffer overflow"));
        }
        use std::io::{Seek, SeekFrom, Write};
        let mut guard = self.file.lock().unwrap();
        guard.seek(SeekFrom::Start(offset as u64))?;
        guard.write_all(data)?;
        Ok(())
    }
}
