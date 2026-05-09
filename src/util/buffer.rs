use std::sync::Arc;

pub trait Reader: Send + Sync {
    fn len(&self) -> usize;
    fn at(&self, i: usize) -> u8;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A slice view into another Reader.
pub struct SliceReader {
    inner: Arc<dyn Reader>,
    beg: usize,
    length: usize,
}

impl SliceReader {
    pub fn new(inner: Arc<dyn Reader>, beg: usize, end: usize) -> Self {
        SliceReader {
            inner,
            beg,
            length: end - beg,
        }
    }
}

impl Reader for SliceReader {
    fn len(&self) -> usize {
        self.length
    }
    fn at(&self, i: usize) -> u8 {
        self.inner.at(self.beg + i)
    }
}

/// In-memory reader backed by a Vec<u8>.
pub struct MemReader {
    data: Vec<u8>,
}

impl MemReader {
    pub fn new(data: Vec<u8>) -> Self {
        MemReader { data }
    }
}

impl Reader for MemReader {
    fn len(&self) -> usize {
        self.data.len()
    }
    fn at(&self, i: usize) -> u8 {
        self.data[i]
    }
}

/// mmap-backed reader using memmap2.
pub struct MmapReader {
    mmap: memmap2::Mmap,
}

impl MmapReader {
    pub fn open(path: &str) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };
        Ok(MmapReader { mmap })
    }
}

impl Reader for MmapReader {
    fn len(&self) -> usize {
        self.mmap.len()
    }
    fn at(&self, i: usize) -> u8 {
        self.mmap[i]
    }
}
