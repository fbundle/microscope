pub mod edit;
pub mod move_;
pub mod render;
pub mod util;

use std::sync::{Arc, Mutex, MutexGuard};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config;
use crate::core::editor::{ActionVal, Cursor, LogEntry, Status, View, Window};
use crate::core::hist::Hist;
use crate::core::text::text::Text;
use crate::util::buffer::Reader;
use crate::util::subscriber_pool::Pool;

/// The insert editor — a Vim-style editor in INSERT mode only.
/// All state is protected by `mu`; a render channel forwards views to the UI.
pub struct InsertEditor {
    pub render_tx: SyncSender<View>,
    pub mu: Mutex<InsertEditorInner>,
}

pub struct InsertEditorInner {
    pub text: Option<Hist<Text>>,
    pub cursor: Cursor,
    pub window: Window,
    pub status: Status,
    pub pool: Pool<Box<dyn Fn(LogEntry) + Send + Sync>>,
}

impl InsertEditor {
    pub fn new(height: usize, width: usize) -> (Self, std::sync::mpsc::Receiver<View>) {
        let cfg = config::load();
        let (tx, rx) = sync_channel(cfg.view_channel_size);
        let editor = InsertEditor {
            render_tx: tx,
            mu: Mutex::new(InsertEditorInner {
                text: None,
                cursor: Cursor { row: 0, col: 0 },
                window: Window { tl_row: 0, tl_col: 0, width, height },
                status: Status::default(),
                pool: Pool::new(),
            }),
        };
        (editor, rx)
    }

    pub fn lock(&self) -> MutexGuard<InsertEditorInner> {
        self.mu.lock().unwrap()
    }

    /// Lock, run f, then send a view to the render channel.
    pub fn lock_render(&self, f: impl FnOnce(&mut InsertEditorInner)) {
        let mut inner = self.mu.lock().unwrap();
        f(&mut inner);
        let view = inner.make_view();
        // non-blocking: if channel full, drop the view (UI will catch up)
        let _ = self.render_tx.try_send(view);
    }

    /// Load a file asynchronously. Returns an AtomicBool that flips to true
    /// when loading is complete.
    pub fn load(
        &self,
        stop_requested: Arc<AtomicBool>,
        reader: Option<Arc<dyn Reader>>,
    ) -> Arc<AtomicBool> {
        {
            let mut inner = self.mu.lock().unwrap();
            if inner.text.is_some() {
                panic!("load called twice");
            }
            inner.text = Some(Hist::new(Text::new(reader.clone())));
            inner.status.background = "loading started".to_string();
            let view = inner.make_view();
            let _ = self.render_tx.try_send(view);
        }

        let done = Arc::new(AtomicBool::new(false));
        let done_clone = done.clone();
        let render_tx = self.render_tx.clone();
        let mu = Arc::new(Mutex::new(()));   // just for signalling; we use self.mu below

        // We can't easily pass &self into a thread, so we clone what we need.
        // The trick: wrap InsertEditorInner in a shared Arc<Mutex<>> externally.
        // Since InsertEditor is not Clone, we use a dedicated thread that locks self.mu.
        // However, the editor itself lives on the main thread's stack.
        // Use a channel to communicate loaded lines.
        let (line_tx, line_rx) = std::sync::mpsc::channel::<(usize, bool, String)>();

        // Loader thread: reads offsets, sends them over a channel
        let reader_clone = reader.clone();
        let stop = stop_requested.clone();
        let load_escape = config::load().load_escape_interval;
        std::thread::spawn(move || {
            if let Some(r) = reader_clone {
                let offsets = crate::core::text::load::index_file(r.as_ref());
                let total = r.len();
                let mut loaded = 0usize;
                for (i, offset) in offsets.iter().enumerate() {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    loaded = *offset;
                    let percent = if total > 0 { 100 * loaded / total } else { 100 };
                    let bg = format!("loading {}/{} ({}%)", loaded, total, percent);
                    let _ = line_tx.send((*offset, false, bg));
                }
                let _ = line_tx.send((0, true, "loaded".to_string()));
            } else {
                let _ = line_tx.send((0, true, "loaded".to_string()));
            }
        });

        // We need to process received offsets on the editor. Since the editor isn't
        // thread-safe by Arc, we use a secondary thread that communicates via the
        // render_tx channel by posting Status updates, and we store lines via a
        // separate Arc-wrapped version of InsertEditorInner.
        //
        // Simpler approach: just do the indexing synchronously in the background thread,
        // collecting all offsets, then apply them. This avoids needing Arc<InsertEditor>.
        // The UI will see "loading started" and then the final state once done.
        // For huge files this is a tradeoff but keeps the architecture clean.
        //
        // To support progress updates properly we need Arc<InsertEditor>.
        // We'll do a minimal approach: collect offsets in background, then apply in a
        // spawned thread that takes ownership of a shared state.
        //
        // Restructure: wrap inner in Arc<Mutex<InsertEditorInner>>.
        // But InsertEditor owns the Mutex directly. We'll send lines through a channel
        // and process them via a dedicated applier that borrows self through a raw pointer.

        // Use unsafe raw pointer approach to allow thread access.
        // Safety: the loading thread will finish before InsertEditor is dropped
        // (we await done_clone before dropping the editor in practice).
        let self_ptr = self as *const InsertEditor as usize;
        let reader_clone2 = reader.clone();
        let done2 = done_clone.clone();

        std::thread::spawn(move || {
            let editor = unsafe { &*(self_ptr as *const InsertEditor) };

            if let Some(r) = &reader_clone2 {
                let offsets = crate::core::text::load::index_file(r.as_ref());
                let total = r.len();
                let cfg = config::load();
                let mut last_pct = usize::MAX;
                let mut last_time = std::time::Instant::now();

                for offset in offsets {
                    if stop_requested.load(Ordering::Relaxed) {
                        break;
                    }

                    let pct = if total > 0 { 100 * offset / total } else { 0 };
                    let now = std::time::Instant::now();
                    let show = pct != last_pct || now.duration_since(last_time) >= cfg.loading_progress_interval;

                    let mut inner = editor.mu.lock().unwrap();
                    if let Some(hist) = &mut inner.text {
                        hist.update(|t| t.append_line(crate::core::text::line::Line::from_offset(offset as u64)));
                    }
                    if show {
                        last_pct = pct;
                        last_time = now;
                        inner.status.background = format!("loading {}/{} ({}%)", offset, total, pct);
                        let view = inner.make_view();
                        let _ = editor.render_tx.try_send(view);
                    }
                }
            }

            // loading done
            {
                let mut inner = editor.mu.lock().unwrap();
                inner.status.background = String::new();
                inner.status.message = "loaded".to_string();
                let view = inner.make_view();
                let _ = editor.render_tx.try_send(view);
            }

            done2.store(true, Ordering::Relaxed);
        });

        done
    }

    pub fn resize(&self, height: usize, width: usize) {
        self.lock_render(|inner| {
            if inner.window.height == height && inner.window.width == width {
                return;
            }
            inner.window.height = height;
            inner.window.width = width;
            inner.move_relative_and_fix(0, 0);
            inner.status.message = format!("resize to {}x{}", height, width);
        });
    }

    pub fn status_update(&self, f: impl Fn(Status) -> Status) {
        self.lock_render(|inner| {
            inner.status = f(inner.status.clone());
        });
    }

    pub fn subscribe(&self, cb: Box<dyn Fn(LogEntry) + Send + Sync>) -> u64 {
        self.mu.lock().unwrap().pool.subscribe(cb)
    }

    pub fn unsubscribe(&self, key: u64) {
        self.mu.lock().unwrap().pool.unsubscribe(key);
    }

    pub fn render_view(&self) -> View {
        self.mu.lock().unwrap().make_view()
    }
}

impl InsertEditorInner {
    pub fn ensure_text(&self) -> &Hist<Text> {
        self.text.as_ref().expect("text not loaded yet")
    }

    pub fn ensure_text_mut(&mut self) -> &mut Hist<Text> {
        self.text.as_mut().expect("text not loaded yet")
    }

    pub fn make_view(&self) -> View {
        let text = self.text.as_ref()
            .map(|h| h.get())
            .unwrap_or_else(|| Text::new(None));
        View {
            text,
            cursor: self.cursor.clone(),
            window: self.window.clone(),
            status: self.status.clone(),
        }
    }

    pub fn write_log(&self, entry: LogEntry) {
        // Dispatch synchronously (Go dispatches via goroutine, same semantics)
        self.pool.for_each(|cb| {
            cb(entry.clone());
        });
    }

    pub fn set_message(&mut self, msg: String) {
        self.status.message = msg;
    }
}
