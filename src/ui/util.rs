use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::editor::LogEntry;
use crate::core::insert_editor::InsertEditor;
use crate::core::log_writer::writer::Writer;
use crate::util::buffer::{MmapReader, Reader};

pub struct Finalizer {
    pub flush: Option<Box<dyn Fn() -> io::Result<()> + Send>>,
    closers: Vec<Box<dyn FnOnce() -> io::Result<()> + Send>>,
}

impl Finalizer {
    pub fn new() -> Self {
        Finalizer { flush: None, closers: vec![] }
    }

    pub fn add_closer(&mut self, f: impl FnOnce() -> io::Result<()> + Send + 'static) {
        self.closers.push(Box::new(f));
    }

    pub fn close(self) {
        for closer in self.closers.into_iter().rev() {
            let _ = closer();
        }
    }

    pub fn flush(&self) -> io::Result<()> {
        if let Some(f) = &self.flush {
            f()
        } else {
            Ok(())
        }
    }
}

pub struct EditorSetup {
    pub insert_editor: Arc<InsertEditor>,
    pub load_done: Arc<AtomicBool>,
    pub rx: std::sync::mpsc::Receiver<crate::core::editor::View>,
}

pub fn make_insert_editor(
    input_filename: &str,
    log_filename: &str,
    width: usize,
    height: usize,
    stop_requested: Arc<AtomicBool>,
) -> io::Result<(EditorSetup, Finalizer)> {
    let (editor, rx) = InsertEditor::new(height, width);
    let editor = Arc::new(editor);
    let mut finalizer = Finalizer::new();

    let reader: Option<Arc<dyn Reader>> = if !input_filename.is_empty() {
        let mmap = MmapReader::open(input_filename)?;
        Some(Arc::new(mmap))
    } else {
        None
    };

    let load_done = editor.load(stop_requested, reader);

    if !log_filename.is_empty() {
        let log_file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(log_filename)?;
        let log_file = std::io::BufWriter::new(log_file);

        // We need a Writer that wraps a BufWriter<File>.
        // Since BufWriter needs flushing, we'll use an Arc<Mutex<BufWriter>> approach.
        use std::sync::Mutex;
        let buf = Arc::new(Mutex::new(log_file));
        let buf2 = buf.clone();

        // Writer takes Box<dyn Write + Send>; we use a wrapper.
        struct LockedWrite(Arc<Mutex<std::io::BufWriter<std::fs::File>>>);
        impl io::Write for LockedWrite {
            fn write(&mut self, b: &[u8]) -> io::Result<usize> {
                self.0.lock().unwrap().write(b)
            }
            fn flush(&mut self) -> io::Result<()> {
                self.0.lock().unwrap().flush()
            }
        }

        let log_writer = Arc::new(
            Writer::new(Box::new(LockedWrite(buf.clone())))?
        );
        let lw2 = log_writer.clone();

        let editor_ref = Arc::clone(&editor);
        editor_ref.subscribe(Box::new(move |entry: LogEntry| {
            let _ = lw2.write(entry);
        }));

        finalizer.flush = Some(Box::new(move || {
            use std::io::Write;
            buf2.lock().unwrap().flush()
        }));
    }

    Ok((EditorSetup { insert_editor: editor, load_done, rx }, finalizer))
}
