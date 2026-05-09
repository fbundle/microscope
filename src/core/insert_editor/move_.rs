use crate::core::editor::Cursor;
use crate::core::insert_editor::{InsertEditor, InsertEditorInner};
use crate::core::text::text::Text;

impl InsertEditorInner {
    pub fn goto_and_fix(&mut self, row: isize, col: isize) {
        let t = self.ensure_text().get();
        let (cur_row, cur_col, tl_row, tl_col) =
            finalize_cursor_and_window(row, col,
                self.window.tl_row as isize,
                self.window.tl_col as isize,
                self.window.width,
                self.window.height,
                &t);

        self.cursor = Cursor { row: cur_row, col: cur_col };
        self.window.tl_row = tl_row;
        self.window.tl_col = tl_col;
    }

    pub fn move_relative_and_fix(&mut self, d_row: isize, d_col: isize) {
        let row = self.cursor.row as isize + d_row;
        let col = self.cursor.col as isize + d_col;
        self.goto_and_fix(row, col);
    }
}

fn finalize_cursor_and_window(
    mut cur_row: isize,
    mut cur_col: isize,
    mut tl_row: isize,
    mut tl_col: isize,
    width: usize,
    height: usize,
    t: &Text,
) -> (usize, usize, usize, usize) {
    // fix cursor according to text
    if t.is_empty() {
        cur_row = 0;
        cur_col = 0;
    } else {
        if cur_row < 0 { cur_row = 0; }
        if cur_row >= t.len() as isize { cur_row = t.len() as isize - 1; }
        if cur_col < 0 { cur_col = 0; }
        let line_len = t.get(cur_row as usize).len() as isize;
        if cur_col > line_len { cur_col = line_len; }
    }

    // fix window according to cursor
    let h = height as isize;
    let w = width as isize;
    if cur_row < tl_row { tl_row = cur_row; }
    if cur_row >= tl_row + h { tl_row = cur_row - h + 1; }
    if cur_col < tl_col { tl_col = cur_col; }
    if cur_col >= tl_col + w { tl_col = cur_col - w + 1; }

    if tl_row < 0 { tl_row = 0; }
    if tl_col < 0 { tl_col = 0; }

    (cur_row as usize, cur_col as usize, tl_row as usize, tl_col as usize)
}

impl InsertEditor {
    pub fn move_left(&self) {
        self.lock_render(|inner| {
            inner.move_relative_and_fix(0, -1);
            inner.set_message("move left".to_string());
        });
    }

    pub fn move_right(&self) {
        self.lock_render(|inner| {
            inner.move_relative_and_fix(0, 1);
            inner.set_message("move right".to_string());
        });
    }

    pub fn move_up(&self) {
        self.lock_render(|inner| {
            inner.move_relative_and_fix(-1, 0);
            inner.set_message("move up".to_string());
        });
    }

    pub fn move_down(&self) {
        self.lock_render(|inner| {
            inner.move_relative_and_fix(1, 0);
            inner.set_message("move down".to_string());
        });
    }

    pub fn move_home(&self) {
        self.lock_render(|inner| {
            let col = inner.cursor.col as isize;
            inner.move_relative_and_fix(0, -col);
            inner.set_message("move home".to_string());
        });
    }

    pub fn move_end(&self) {
        self.lock_render(|inner| {
            let t = inner.ensure_text().get();
            let row = inner.cursor.row;
            if row < t.len() {
                let line_len = t.get(row).len() as isize;
                let col = inner.cursor.col as isize;
                inner.move_relative_and_fix(0, line_len - col);
            }
            inner.set_message("move end".to_string());
        });
    }

    pub fn move_page_up(&self) {
        self.lock_render(|inner| {
            let h = inner.window.height as isize;
            inner.move_relative_and_fix(-h, 0);
            inner.set_message("move page up".to_string());
        });
    }

    pub fn move_page_down(&self) {
        self.lock_render(|inner| {
            let h = inner.window.height as isize;
            inner.move_relative_and_fix(h, 0);
            inner.set_message("move page down".to_string());
        });
    }

    pub fn goto(&self, row: usize, col: usize) {
        self.lock_render(|inner| {
            inner.goto_and_fix(row as isize, col as isize);
            inner.set_message(format!("goto ({}, {})", row + 1, col + 1));
        });
    }
}
