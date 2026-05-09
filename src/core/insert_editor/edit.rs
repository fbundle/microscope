use crate::core::editor::{Command, LogEntry};
use crate::core::insert_editor::InsertEditor;
use crate::core::text::text::Text;

use super::util::{concat_slices, delete_from_slice, insert_to_slice};

impl InsertEditor {
    pub fn type_char(&self, ch: char) {
        self.lock_render(|inner| {
            let row = inner.cursor.row;
            let col = inner.cursor.col;
            inner.write_log(LogEntry {
                command: Command::Type,
                row: row as u64,
                col: col as u64,
                rune: ch,
                ..Default::default()
            });

            inner.ensure_text_mut().update(|t| {
                if t.is_empty() {
                    return t.ins(0, vec![ch]);
                }
                let mut line = t.get(row);
                insert_to_slice(&mut line, col, ch);
                t.set(row, line)
            });
            inner.move_relative_and_fix(0, 1);
            inner.set_message(format!("type '{}'", ch));
        });
    }

    pub fn backspace(&self) {
        self.lock_render(|inner| {
            let row = inner.cursor.row;
            let col = inner.cursor.col;
            inner.write_log(LogEntry {
                command: Command::Backspace,
                row: row as u64,
                col: col as u64,
                ..Default::default()
            });

            let mut move_row: isize = 0;
            let mut move_col: isize = 0;

            inner.ensure_text_mut().update(|t| {
                if t.is_empty() {
                    return t;
                }
                if col == 0 && row == 0 {
                    // do nothing
                } else if col == 0 {
                    // merge current line into previous
                    let line1 = t.get(row - 1);
                    let line2 = t.get(row);
                    let merged = concat_slices(&[&line1, &line2]);
                    let len1 = line1.len();
                    let t = t.set(row - 1, merged).del(row);
                    move_row = -1;
                    move_col = len1 as isize;
                    return t;
                } else {
                    let mut line = t.get(row);
                    delete_from_slice(&mut line, col - 1);
                    move_col = -1;
                    return t.set(row, line);
                }
                t
            });

            inner.move_relative_and_fix(move_row, move_col);
            inner.set_message("backspace".to_string());
        });
    }

    pub fn delete(&self) {
        self.lock_render(|inner| {
            let row = inner.cursor.row;
            let col = inner.cursor.col;
            inner.write_log(LogEntry {
                command: Command::Delete,
                row: row as u64,
                col: col as u64,
                ..Default::default()
            });

            inner.ensure_text_mut().update(|t| {
                if t.is_empty() {
                    return t;
                }
                let line1 = t.get(row);
                if col == line1.len() && row == t.len() - 1 {
                    // last line, do nothing
                } else if col == line1.len() {
                    // merge next line
                    let line2 = t.get(row + 1);
                    let merged = concat_slices(&[&line1, &line2]);
                    return t.set(row, merged).del(row + 1);
                } else {
                    let mut line = line1;
                    delete_from_slice(&mut line, col);
                    return t.set(row, line);
                }
                t
            });

            inner.set_message("delete".to_string());
        });
    }

    pub fn enter(&self) {
        self.lock_render(|inner| {
            let row = inner.cursor.row;
            let col = inner.cursor.col;
            inner.write_log(LogEntry {
                command: Command::Enter,
                row: row as u64,
                col: col as u64,
                ..Default::default()
            });

            inner.ensure_text_mut().update(|t| {
                if t.is_empty() {
                    return t.ins(0, vec![]);
                }
                let line = t.get(row);
                if col == line.len() {
                    t.ins(row + 1, vec![])
                } else {
                    let line1 = line[..col].to_vec();
                    let line2 = line[col..].to_vec();
                    t.set(row, line1).ins(row + 1, line2)
                }
            });

            inner.move_relative_and_fix(1, 0);
            inner.move_relative_and_fix(0, -(inner.cursor.col as isize));
            inner.set_message("enter".to_string());
        });
    }

    pub fn undo(&self) {
        self.lock_render(|inner| {
            inner.write_log(LogEntry { command: Command::Undo, ..Default::default() });
            inner.ensure_text_mut().undo();
            inner.move_relative_and_fix(0, 0);
            inner.set_message("undo".to_string());
        });
    }

    pub fn redo(&self) {
        self.lock_render(|inner| {
            inner.write_log(LogEntry { command: Command::Redo, ..Default::default() });
            inner.ensure_text_mut().redo();
            inner.move_relative_and_fix(0, 0);
            inner.set_message("redo".to_string());
        });
    }

    pub fn insert_line(&self, t2: Text) {
        self.lock_render(|inner| {
            let row = inner.cursor.row;
            inner.write_log(LogEntry {
                command: Command::InsertLine,
                row: row as u64,
                text: t2.repr(),
                ..Default::default()
            });

            let t2_len = t2.len();
            inner.ensure_text_mut().update(|t| {
                Text::merge(&[
                    Text::slice(&t, 0, row),
                    t2.clone(),
                    Text::slice(&t, row, t.len()),
                ])
            });
            inner.move_relative_and_fix(t2_len as isize, 0);
            inner.set_message("insert lines".to_string());
        });
    }

    pub fn delete_line(&self, count: usize) {
        self.lock_render(|inner| {
            let row = inner.cursor.row;
            inner.write_log(LogEntry {
                command: Command::DeleteLine,
                row: row as u64,
                count: count as u64,
                ..Default::default()
            });

            inner.ensure_text_mut().update(|t| {
                let end = (row + count).min(t.len());
                Text::merge(&[
                    Text::slice(&t, 0, row),
                    Text::slice(&t, end, t.len()),
                ])
            });
            inner.set_message("delete lines".to_string());
        });
    }

    pub fn apply(&self, entry: LogEntry) {
        match entry.command {
            Command::Enter => {
                self.goto(entry.row as usize, entry.col as usize);
                self.enter();
            }
            Command::Backspace => {
                self.goto(entry.row as usize, entry.col as usize);
                self.backspace();
            }
            Command::Delete => {
                self.goto(entry.row as usize, entry.col as usize);
                self.delete();
            }
            Command::Type => {
                self.goto(entry.row as usize, entry.col as usize);
                self.type_char(entry.rune);
            }
            Command::Undo => self.undo(),
            Command::Redo => self.redo(),
            Command::InsertLine => {
                self.goto(entry.row as usize, 0);
                self.insert_line(Text::make_from_lines(entry.text));
            }
            Command::DeleteLine => {
                self.goto(entry.row as usize, 0);
                self.delete_line(entry.count as usize);
            }
            Command::SetVersion => {}
        }
    }
}

impl Default for LogEntry {
    fn default() -> Self {
        LogEntry {
            command: Command::Type,
            version: 0,
            row: 0,
            col: 0,
            rune: '\0',
            text: vec![],
            count: 0,
            beg: 0,
            end: 0,
        }
    }
}
