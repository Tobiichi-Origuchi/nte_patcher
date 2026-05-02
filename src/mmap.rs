#![allow(missing_docs)]
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
