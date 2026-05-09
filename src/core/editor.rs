use std::collections::HashMap;
use std::sync::Arc;

use crate::core::text::text::Text;
use crate::util::buffer::Reader;

#[derive(Clone, Debug, Default)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Status {
    pub message: String,
    pub background: String,
    pub other: Option<HashMap<String, OtherValue>>,
}

#[derive(Clone, Debug)]
pub enum OtherValue {
    Str(String),
    Selector(Selector),
}

#[derive(Clone, Debug)]
pub struct Selector {
    pub beg: usize,
    pub end: usize,
}

impl Selector {
    pub fn interval(&self) -> (usize, usize) {
        if self.beg <= self.end {
            (self.beg, self.end)
        } else {
            (self.end, self.beg)
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Window {
    pub tl_row: usize,
    pub tl_col: usize,
    pub width: usize,
    pub height: usize,
}

#[derive(Clone)]
pub struct View {
    pub text: Text,
    pub cursor: Cursor,
    pub window: Window,
    pub status: Status,
}

// ---------------------------------------------------------------------------
// Command / LogEntry
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Command {
    #[serde(rename = "set_version")]
    SetVersion,
    #[serde(rename = "type")]
    Type,
    #[serde(rename = "enter")]
    Enter,
    #[serde(rename = "backspace")]
    Backspace,
    #[serde(rename = "delete")]
    Delete,
    #[serde(rename = "undo")]
    Undo,
    #[serde(rename = "redo")]
    Redo,
    #[serde(rename = "insert_line")]
    InsertLine,
    #[serde(rename = "delete_line")]
    DeleteLine,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LogEntry {
    pub command: Command,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub version: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub row: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub col: u64,
    #[serde(default, skip_serializing_if = "is_zero_char")]
    pub rune: char,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub text: Vec<Vec<char>>,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub count: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub beg: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub end: u64,
}

fn is_zero_u64(v: &u64) -> bool { *v == 0 }
fn is_zero_char(v: &char) -> bool { *v == '\0' }

// ---------------------------------------------------------------------------
// Editor trait
// ---------------------------------------------------------------------------

pub trait EditorMove {
    fn move_left(&mut self);
    fn move_right(&mut self);
    fn move_up(&mut self);
    fn move_down(&mut self);
    fn move_home(&mut self);
    fn move_end(&mut self);
    fn move_page_up(&mut self);
    fn move_page_down(&mut self);
    fn goto(&mut self, row: usize, col: usize);
}

pub trait EditorEdit {
    fn type_char(&mut self, ch: char);
    fn backspace(&mut self);
    fn delete(&mut self);
    fn enter(&mut self);
    fn undo(&mut self);
    fn redo(&mut self);
    fn insert_line(&mut self, t2: Text);
    fn delete_line(&mut self, count: usize);
    fn apply(&mut self, entry: LogEntry);
}

pub trait EditorRender {
    fn render(&self) -> View;
    fn update(&self) -> std::sync::mpsc::Receiver<View>;
}

pub trait Editor: EditorMove + EditorEdit + Send + Sync {
    fn load(
        &mut self,
        ctx_cancel: Arc<dyn Fn() + Send + Sync>,
        reader: Option<Arc<dyn Reader>>,
    ) -> std::io::Result<std::sync::Arc<std::sync::atomic::AtomicBool>>;

    fn resize(&mut self, height: usize, width: usize);
    fn status_update(&mut self, f: &dyn Fn(Status) -> Status);
    fn action(&mut self, key: &str, vals: &[ActionVal]);
    fn subscribe(&mut self, cb: Box<dyn Fn(LogEntry) + Send + Sync>) -> u64;
    fn unsubscribe(&mut self, key: u64);
    fn render(&self) -> View;
}

#[derive(Clone, Debug)]
pub enum ActionVal {
    CursorPos(Cursor),
    Str(String),
}
