use crate::util::buffer::Reader;

/// Yield the byte offset of the start of each line in the reader.
/// Scans for '\n'; yields offset before each newline, and the final offset
/// if the file doesn't end with a newline.
pub fn index_file(reader: &dyn Reader) -> Vec<usize> {
    let mut offsets = Vec::new();
    let mut offset = 0usize;
    let len = reader.len();
    for i in 0..len {
        let b = reader.at(i);
        if b == b'\n' {
            offsets.push(offset);
            offset = i + 1;
        }
    }
    if offset < len {
        offsets.push(offset);
    }
    offsets
}
