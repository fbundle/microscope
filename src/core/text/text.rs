use std::sync::Arc;

use crate::core::text::line::Line;
use crate::util::buffer::Reader;
use crate::util::persistent::seq::{merge_seqs, Seq};

fn chars_to_bytes(chars: &[char]) -> Vec<u8> {
    let s: String = chars.iter().collect();
    s.into_bytes()
}

fn bytes_to_chars(bytes: &[u8]) -> Vec<char> {
    String::from_utf8_lossy(bytes).chars().collect()
}

/// A persistent text buffer (sequence of lines).
#[derive(Clone)]
pub struct Text {
    pub reader: Option<Arc<dyn Reader>>,
    pub lines: Seq<Line>,
}

impl Text {
    pub fn new(reader: Option<Arc<dyn Reader>>) -> Self {
        Text { reader, lines: Seq::empty() }
    }

    pub fn get(&self, i: usize) -> Vec<char> {
        let line = self.lines.get(i);
        match &self.reader {
            Some(r) => bytes_to_chars(&line.repr(r.as_ref())),
            None => bytes_to_chars(&line.repr(&crate::util::buffer::MemReader::new(vec![]))),
        }
    }

    fn reader_ref(&self) -> Option<&dyn Reader> {
        self.reader.as_deref()
    }

    pub fn get_bytes(&self, i: usize) -> Vec<u8> {
        let line = self.lines.get(i);
        match &self.reader {
            Some(r) => line.repr(r.as_ref()),
            None => line.repr(&crate::util::buffer::MemReader::new(vec![])),
        }
    }

    pub fn set(&self, i: usize, val: Vec<char>) -> Text {
        Text {
            reader: self.reader.clone(),
            lines: self.lines.set(i, Line::from_data(chars_to_bytes(&val))),
        }
    }

    pub fn ins(&self, i: usize, val: Vec<char>) -> Text {
        Text {
            reader: self.reader.clone(),
            lines: self.lines.ins(i, Line::from_data(chars_to_bytes(&val))),
        }
    }

    pub fn append_line(&self, line: Line) -> Text {
        let n = self.lines.len();
        Text {
            reader: self.reader.clone(),
            lines: self.lines.ins(n, line),
        }
    }

    pub fn del(&self, i: usize) -> Text {
        Text {
            reader: self.reader.clone(),
            lines: self.lines.del(i),
        }
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn iter_lines(&self) -> Vec<Vec<char>> {
        (0..self.len()).map(|i| self.get(i)).collect()
    }

    pub fn repr(&self) -> Vec<Vec<char>> {
        self.iter_lines()
    }

    pub fn slice(t: &Text, beg: usize, end: usize) -> Text {
        let (_, right) = t.lines.split(beg);
        let (middle, _) = right.split(end - beg);
        Text { reader: t.reader.clone(), lines: middle }
    }

    pub fn merge(texts: &[Text]) -> Text {
        if texts.is_empty() {
            return Text::new(None);
        }
        let reader = texts.iter().find_map(|t| t.reader.clone());
        let seqs: Vec<_> = texts.iter().map(|t| t.lines.clone()).collect();
        let merged = merge_seqs(&seqs);
        Text { reader, lines: merged }
    }

    pub fn make_from_lines(lines: Vec<Vec<char>>) -> Text {
        let mut seq = Seq::empty();
        for line in lines {
            let n = seq.len();
            seq = seq.ins(n, Line::from_data(chars_to_bytes(&line)));
        }
        Text { reader: None, lines: seq }
    }
}
