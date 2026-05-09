// 8-byte Line type using tagged pointer.
// bit 0 = 0: file offset (actual offset = value >> 1). Supports up to 2^62 bytes.
// bit 0 = 1: Arc<Vec<u8>> pointer with bit 0 set.

use std::sync::Arc;

const IN_MEMORY_TAG: u64 = 1;

/// A line packed into 8 bytes.
#[repr(transparent)]
pub struct Line(u64);

impl Line {
    pub fn from_offset(offset: u64) -> Self {
        Line(offset << 1)
    }

    pub fn from_data(data: Vec<u8>) -> Self {
        let arc = Arc::new(data);
        let ptr = Arc::into_raw(arc) as u64;
        debug_assert_eq!(ptr & IN_MEMORY_TAG, 0, "Arc pointer must be even-aligned");
        Line(ptr | IN_MEMORY_TAG)
    }

    pub fn is_in_memory(&self) -> bool {
        self.0 & IN_MEMORY_TAG != 0
    }

    pub fn offset(&self) -> u64 {
        self.0 >> 1
    }

    fn arc_ptr(&self) -> *const Vec<u8> {
        (self.0 & !IN_MEMORY_TAG) as *const Vec<u8>
    }

    pub fn repr(&self, reader: &dyn crate::util::buffer::Reader) -> Vec<u8> {
        if self.is_in_memory() {
            let arc = unsafe { Arc::from_raw(self.arc_ptr()) };
            let data = arc.as_ref().clone();
            std::mem::forget(arc);
            data
        } else {
            let offset = self.offset() as usize;
            let len = reader.len();
            let mut buf = Vec::new();
            let mut i = offset;
            while i < len {
                let b = reader.at(i);
                if b == b'\n' {
                    break;
                }
                buf.push(b);
                i += 1;
            }
            buf
        }
    }
}

impl Clone for Line {
    fn clone(&self) -> Self {
        if self.is_in_memory() {
            let arc = unsafe { Arc::from_raw(self.arc_ptr()) };
            let cloned = Arc::clone(&arc);
            std::mem::forget(arc);
            Line(Arc::into_raw(cloned) as u64 | IN_MEMORY_TAG)
        } else {
            Line(self.0)
        }
    }
}

impl Drop for Line {
    fn drop(&mut self) {
        if self.is_in_memory() {
            unsafe {
                drop(Arc::from_raw(self.arc_ptr()));
            }
        }
    }
}

unsafe impl Send for Line {}
unsafe impl Sync for Line {}
