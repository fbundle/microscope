pub mod mode_change;

use std::sync::{Arc, Mutex};

use crate::core::editor::{ActionVal, Cursor, LogEntry, OtherValue, Selector, Status, View};
use crate::core::insert_editor::InsertEditor;
use crate::core::text::text::Text;
use crate::config;

pub const MODE_NORMAL: &str = "NORMAL";
pub const MODE_INSERT: &str = "INSERT";
pub const MODE_COMMAND: &str = "COMMAND";
pub const MODE_SELECT: &str = "SELECT";

pub struct MultiModeEditor {
    pub stop: Arc<dyn Fn() + Send + Sync>,
    pub mu: Mutex<MultiModeState>,
    pub inner: Arc<InsertEditor>,
    pub default_output_file: String,
}

pub struct MultiModeState {
    pub mode: String,
    pub command: String,
    pub selector: Option<Selector>,
    pub clipboard: Text,
}

impl MultiModeState {
    fn enter_normal(&mut self) {
        self.mode = MODE_NORMAL.to_string();
        self.command = String::new();
        self.selector = None;
    }
    fn enter_insert(&mut self) {
        self.mode = MODE_INSERT.to_string();
        self.command = String::new();
        self.selector = None;
    }
    fn enter_command(&mut self, cmd: String) {
        self.mode = MODE_COMMAND.to_string();
        self.command = cmd;
        self.selector = None;
    }
    fn enter_select(&mut self, beg: usize) {
        self.mode = MODE_SELECT.to_string();
        self.command = String::new();
        self.selector = Some(Selector { beg, end: beg });
    }
}

impl MultiModeEditor {
    pub fn new(
        inner: Arc<InsertEditor>,
        stop: Arc<dyn Fn() + Send + Sync>,
        default_output_file: String,
    ) -> Self {
        let editor = MultiModeEditor {
            stop,
            mu: Mutex::new(MultiModeState {
                mode: MODE_NORMAL.to_string(),
                command: String::new(),
                selector: None,
                clipboard: Text::new(None),
            }),
            inner,
            default_output_file,
        };
        editor.write_status("");
        editor
    }

    pub fn write_status(&self, message: &str) {
        let state = self.mu.lock().unwrap();
        let mode = state.mode.clone();
        let command = state.command.clone();
        let selector = state.selector.clone();
        let msg = message.to_string();
        drop(state);

        self.inner.status_update(|mut status| {
            let map = status.other.get_or_insert_with(Default::default);
            map.insert("mode".to_string(), OtherValue::Str(mode.clone()));
            map.insert("command".to_string(), OtherValue::Str(command.clone()));
            if let Some(sel) = &selector {
                map.insert("selector".to_string(), OtherValue::Selector(sel.clone()));
            } else {
                map.remove("selector");
            }
            status.message = msg.clone();
            status
        });
    }

    pub fn render_view(&self) -> View {
        self.inner.get_view()
    }

    fn maybe_update_selector_end(&self, state: &mut MultiModeState) {
        if state.mode == MODE_SELECT {
            let view = self.inner.get_view();
            state.selector.as_mut().map(|s| s.end = view.cursor.row);
        }
    }

    pub fn move_left(&self) {
        self.inner.move_left();
    }

    pub fn move_right(&self) {
        self.inner.move_right();
    }

    pub fn move_up(&self) {
        self.inner.move_up();
        let mut state = self.mu.lock().unwrap();
        self.maybe_update_selector_end(&mut state);
        if state.mode == MODE_SELECT {
            drop(state);
            self.write_status("select more");
        }
    }

    pub fn move_down(&self) {
        self.inner.move_down();
        let mut state = self.mu.lock().unwrap();
        self.maybe_update_selector_end(&mut state);
        if state.mode == MODE_SELECT {
            drop(state);
            self.write_status("select more");
        }
    }

    pub fn move_home(&self) {
        self.inner.move_home();
    }

    pub fn move_end(&self) {
        self.inner.move_end();
    }

    pub fn move_page_up(&self) {
        self.inner.move_page_up();
        let mut state = self.mu.lock().unwrap();
        self.maybe_update_selector_end(&mut state);
        if state.mode == MODE_SELECT {
            drop(state);
            self.write_status("select more");
        }
    }

    pub fn move_page_down(&self) {
        self.inner.move_page_down();
        let mut state = self.mu.lock().unwrap();
        self.maybe_update_selector_end(&mut state);
        if state.mode == MODE_SELECT {
            drop(state);
            self.write_status("select more");
        }
    }

    pub fn goto_pos(&self, row: usize, col: usize) {
        self.inner.goto(row, col);
    }

    pub fn undo(&self) {
        self.inner.undo();
    }

    pub fn redo(&self) {
        self.inner.redo();
    }

    pub fn apply(&self, entry: LogEntry) {
        self.inner.apply(entry);
    }

    pub fn insert_line(&self, t2: Text) {
        self.inner.insert_line(t2);
    }

    pub fn delete_line(&self, count: usize) {
        self.inner.delete_line(count);
    }

    pub fn subscribe(&self, cb: Box<dyn Fn(LogEntry) + Send + Sync>) -> u64 {
        self.inner.subscribe(cb)
    }

    pub fn unsubscribe(&self, key: u64) {
        self.inner.unsubscribe(key);
    }
}
