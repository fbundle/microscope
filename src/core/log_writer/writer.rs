use std::io::{self, Write};
use std::sync::Mutex;

use crate::config;
use crate::core::editor::{Command, LogEntry};
use crate::core::log_writer::serializer::{get_serializer, Serializer};

fn length_prefix_write(w: &mut dyn Write, b: &[u8]) -> io::Result<()> {
    let len_bytes = (b.len() as u64).to_le_bytes();
    w.write_all(&len_bytes)?;
    w.write_all(b)?;
    Ok(())
}

pub struct Writer {
    mu: Mutex<WriterInner>,
}

struct WriterInner {
    writer: Box<dyn Write + Send>,
    serializer: Box<dyn Serializer>,
}

impl Writer {
    pub fn new(writer: Box<dyn Write + Send>) -> io::Result<Self> {
        let cfg = config::load();
        let initial_ser = get_serializer(cfg.initial_serializer_version)?;
        let target_ser = get_serializer(cfg.serializer_version)?;

        let mut inner = WriterInner {
            writer,
            serializer: initial_ser,
        };

        // Write set_version entry using initial serializer
        let set_ver = LogEntry {
            command: Command::SetVersion,
            version: cfg.serializer_version,
            ..Default::default()
        };
        let b = inner.serializer.marshal(&set_ver)?;
        length_prefix_write(&mut *inner.writer, &b)?;

        // Switch to target serializer
        inner.serializer = target_ser;

        Ok(Writer { mu: Mutex::new(inner) })
    }

    pub fn write(&self, entry: LogEntry) -> io::Result<()> {
        let mut inner = self.mu.lock().unwrap();
        let b = inner.serializer.marshal(&entry)?;
        length_prefix_write(&mut *inner.writer, &b)
    }

    pub fn flush(&self) -> io::Result<()> {
        // If the writer supports flush, we'd call it here.
        // Since we use a Box<dyn Write>, we can't call flush directly unless we add it.
        // The caller should flush the underlying writer.
        Ok(())
    }
}
