use crate::config;
use crate::core::editor::{ActionVal, Cursor};
use crate::core::multimode_editor::{
    MultiModeEditor, MODE_COMMAND, MODE_INSERT, MODE_NORMAL, MODE_SELECT,
};
use crate::core::text::text::Text;
use crate::util::file_util;
use regex::Regex;

impl MultiModeEditor {
    pub fn key_escape(&self) {
        {
            let mut state = self.mu.lock().unwrap();
            state.enter_normal();
        }
        self.write_status("");
    }

    pub fn enter(&self) {
        let mode = self.mu.lock().unwrap().mode.clone();
        match mode.as_str() {
            MODE_NORMAL => {}
            MODE_INSERT => {
                self.inner.enter();
            }
            MODE_COMMAND => {
                self.apply_command();
            }
            MODE_SELECT => {}
            _ => panic!("unknown mode: {}", mode),
        }
    }

    pub fn backspace(&self) {
        let mode = self.mu.lock().unwrap().mode.clone();
        match mode.as_str() {
            MODE_NORMAL => {}
            MODE_INSERT => {
                self.inner.backspace();
            }
            MODE_COMMAND => {
                let mut state = self.mu.lock().unwrap();
                if !state.command.is_empty() {
                    // pop last char
                    let mut chars: Vec<char> = state.command.chars().collect();
                    chars.pop();
                    state.command = chars.into_iter().collect();
                    if state.command.is_empty() {
                        state.enter_normal();
                    }
                }
                drop(state);
                self.write_status("");
            }
            MODE_SELECT => {}
            _ => panic!("unknown mode: {}", mode),
        }
    }

    pub fn delete(&self) {
        let mode = self.mu.lock().unwrap().mode.clone();
        match mode.as_str() {
            MODE_NORMAL => {
                {
                    let mut state = self.mu.lock().unwrap();
                    state.enter_insert();
                }
                self.write_status("");
                self.inner.delete();
            }
            MODE_INSERT => {
                self.inner.delete();
            }
            MODE_COMMAND | MODE_SELECT => {}
            _ => panic!("unknown mode: {}", mode),
        }
    }

    pub fn type_char(&self, ch: char) {
        let mode = self.mu.lock().unwrap().mode.clone();
        match mode.as_str() {
            MODE_NORMAL => {
                match ch {
                    'i' => {
                        {
                            let mut state = self.mu.lock().unwrap();
                            state.enter_insert();
                        }
                        self.write_status("");
                    }
                    ':' | '/' => {
                        {
                            let mut state = self.mu.lock().unwrap();
                            state.enter_command(ch.to_string());
                        }
                        self.write_status("");
                    }
                    'V' => {
                        let row = self.inner.get_view().cursor.row;
                        {
                            let mut state = self.mu.lock().unwrap();
                            state.enter_select(row);
                        }
                        self.write_status("");
                    }
                    'p' => {
                        let (is_empty, clipboard) = {
                            let state = self.mu.lock().unwrap();
                            (state.clipboard.is_empty(), state.clipboard.clone())
                        };
                        if is_empty {
                            self.write_status("clipboard is empty");
                        } else {
                            self.inner.insert_line(clipboard);
                            self.write_status("pasted");
                        }
                    }
                    'u' => self.inner.undo(),
                    'r' => self.inner.redo(),
                    'b' | 'g' => {
                        self.inner.goto(0, 0);
                    }
                    'e' | 'G' => {
                        let last = self.inner.get_view().text.len().saturating_sub(1);
                        self.inner.goto(last, 0);
                    }
                    _ => {}
                }
            }
            MODE_INSERT => {
                self.inner.type_char(ch);
            }
            MODE_COMMAND => {
                {
                    let mut state = self.mu.lock().unwrap();
                    state.command.push(ch);
                }
                self.write_status("");
            }
            MODE_SELECT => {
                match ch {
                    'd' => {
                        // cut
                        let (beg, end) = {
                            let state = self.mu.lock().unwrap();
                            state.selector.as_ref().map(|s| s.interval()).unwrap_or((0, 0))
                        };
                        let t = self.inner.get_view().text;
                        let clipboard = Text::slice(&t, beg, end + 1);
                        {
                            let mut state = self.mu.lock().unwrap();
                            state.clipboard = clipboard;
                            state.enter_normal();
                        }
                        self.inner.goto(beg, 0);
                        let count = end - beg + 1;
                        self.inner.delete_line(count);
                        self.write_status("cut");
                    }
                    'y' => {
                        // copy
                        let (beg, end) = {
                            let state = self.mu.lock().unwrap();
                            state.selector.as_ref().map(|s| s.interval()).unwrap_or((0, 0))
                        };
                        let t = self.inner.get_view().text;
                        let clipboard = Text::slice(&t, beg, end + 1);
                        {
                            let mut state = self.mu.lock().unwrap();
                            state.clipboard = clipboard;
                            state.enter_normal();
                        }
                        self.write_status("copied");
                    }
                    'b' | 'g' => {
                        self.inner.goto(0, 0);
                        let mut state = self.mu.lock().unwrap();
                        let row = self.inner.get_view().cursor.row;
                        if let Some(sel) = &mut state.selector { sel.end = row; }
                        drop(state);
                        self.write_status("select more");
                    }
                    'e' | 'G' => {
                        let last = self.inner.get_view().text.len().saturating_sub(1);
                        self.inner.goto(last, 0);
                        let mut state = self.mu.lock().unwrap();
                        if let Some(sel) = &mut state.selector { sel.end = last; }
                        drop(state);
                        self.write_status("select more");
                    }
                    _ => {}
                }
            }
            _ => panic!("unknown mode: {}", mode),
        }
    }

    pub fn action(&self, key: &str, vals: &[ActionVal]) {
        match key {
            "mouse_click_left" => {
                let mode = self.mu.lock().unwrap().mode.clone();
                if mode == MODE_INSERT {
                    if let Some(ActionVal::CursorPos(p)) = vals.get(0) {
                        let view = self.inner.get_view();
                        let row = view.window.tl_row + p.row;
                        let col = view.window.tl_col + p.col;
                        self.inner.goto(row, col);
                    }
                }
            }
            "mouse_scroll_up" => {
                let speed = config::load().scroll_speed;
                for _ in 0..speed { self.inner.move_up(); }
            }
            "mouse_scroll_down" => {
                let speed = config::load().scroll_speed;
                for _ in 0..speed { self.inner.move_down(); }
            }
            "mouse_scroll_left" => {
                let speed = config::load().scroll_speed;
                for _ in 0..speed { self.inner.move_left(); }
            }
            "mouse_scroll_right" => {
                let speed = config::load().scroll_speed;
                for _ in 0..speed { self.inner.move_right(); }
            }
            "key_escape" => {
                self.key_escape();
            }
            "key_tabular" => {
                let mode = self.mu.lock().unwrap().mode.clone();
                if mode == MODE_INSERT {
                    let tab_size = config::load().tab_size;
                    for _ in 0..tab_size {
                        self.inner.type_char(' ');
                    }
                }
            }
            _ => {
                self.write_status(&format!("action not supported: {}", key));
            }
        }
    }

    fn apply_command(&self) {
        let cmd_str = self.mu.lock().unwrap().command.clone();
        let (cmd, args) = parse_command(&cmd_str);

        match cmd {
            ParsedCommand::Insert => {
                {
                    let mut state = self.mu.lock().unwrap();
                    state.enter_insert();
                }
                self.write_status("");
            }
            ParsedCommand::Quit => {
                (self.stop)();
            }
            ParsedCommand::WriteQuit => {
                let filename = self.default_output_file.clone();
                let view = self.inner.get_view();
                let iter: Vec<Vec<char>> = view.text.iter_lines();
                let result = file_util::safe_write_file(&filename, iter.into_iter());
                {
                    let mut state = self.mu.lock().unwrap();
                    state.enter_normal();
                }
                match result {
                    Ok(_) => self.write_status(&format!("file written into {}", filename)),
                    Err(e) => self.write_status(&format!("error write file {}", e)),
                }
                (self.stop)();
            }
            ParsedCommand::Search | ParsedCommand::Regex => {
                if args.is_empty() {
                    let mut state = self.mu.lock().unwrap();
                    state.enter_normal();
                    drop(state);
                    self.write_status("empty args");
                    return;
                }
                let pattern = &args[0];
                let matcher: Box<dyn Fn(&str) -> bool> = if cmd == ParsedCommand::Regex {
                    match Regex::new(pattern) {
                        Ok(re) => Box::new(move |line: &str| re.is_match(line)),
                        Err(e) => {
                            let mut state = self.mu.lock().unwrap();
                            state.enter_normal();
                            drop(state);
                            self.write_status(&format!("regexp compile error {}", e));
                            return;
                        }
                    }
                } else {
                    let pat = pattern.clone();
                    Box::new(move |line: &str| line.contains(&pat as &str))
                };

                let view = self.inner.get_view();
                let start_row = view.cursor.row;
                let t = view.text;
                let total = t.len();
                let cfg = config::load();
                let t0 = std::time::Instant::now();
                let mut found = false;
                for i in (start_row + 1)..total {
                    let line: String = t.get(i).into_iter().collect();
                    if matcher(&line) {
                        self.inner.goto(i, 0);
                        let mut state = self.mu.lock().unwrap();
                        state.enter_normal();
                        drop(state);
                        self.write_status(&format!("found substring {}", pattern));
                        found = true;
                        break;
                    }
                    if t0.elapsed() > cfg.max_search_time {
                        self.inner.goto(i, 0);
                        let mut state = self.mu.lock().unwrap();
                        state.enter_normal();
                        drop(state);
                        self.write_status(&format!(
                            "search timeout after {} seconds and {} entries",
                            cfg.max_search_time.as_secs(),
                            i - start_row
                        ));
                        found = true;
                        break;
                    }
                }
                if !found {
                    if total > 0 { self.inner.goto(total - 1, 0); }
                    let mut state = self.mu.lock().unwrap();
                    state.enter_normal();
                    drop(state);
                    self.write_status("end of file");
                }
            }
            ParsedCommand::Goto => {
                if args.is_empty() {
                    let mut state = self.mu.lock().unwrap();
                    state.enter_normal();
                    drop(state);
                    self.write_status("empty args");
                    return;
                }
                match args[0].parse::<usize>() {
                    Ok(line_num) => {
                        self.inner.goto(line_num.saturating_sub(1), 0);
                        let mut state = self.mu.lock().unwrap();
                        state.enter_normal();
                        drop(state);
                        self.write_status(&format!("goto line {}", args[0]));
                    }
                    Err(_) => {
                        let mut state = self.mu.lock().unwrap();
                        state.enter_normal();
                        drop(state);
                        self.write_status(&format!("invalid line number {}", args[0]));
                    }
                }
            }
            ParsedCommand::Write => {
                if args.is_empty() {
                    let mut state = self.mu.lock().unwrap();
                    state.enter_normal();
                    drop(state);
                    self.write_status("empty args");
                    return;
                }
                let filename = args[0].clone();
                let view = self.inner.get_view();
                let iter: Vec<Vec<char>> = view.text.iter_lines();
                let result = file_util::safe_write_file(&filename, iter.into_iter());
                let mut state = self.mu.lock().unwrap();
                state.enter_normal();
                drop(state);
                match result {
                    Ok(_) => self.write_status(&format!("file written into {}", filename)),
                    Err(e) => self.write_status(&format!("error write file {}", e)),
                }
            }
            ParsedCommand::Unknown => {
                let mut state = self.mu.lock().unwrap();
                state.enter_normal();
                drop(state);
                self.write_status(&format!("unknown command: {}", cmd_str));
            }
        }
    }
}

#[derive(PartialEq)]
enum ParsedCommand {
    Insert,
    Quit,
    WriteQuit,
    Search,
    Regex,
    Goto,
    Write,
    Unknown,
}

fn parse_command(cmd: &str) -> (ParsedCommand, Vec<String>) {
    if cmd == ":i" || cmd == ":insert" {
        return (ParsedCommand::Insert, vec![]);
    }
    if cmd == ":q" || cmd == ":quit" || cmd == ":q!" {
        return (ParsedCommand::Quit, vec![]);
    }
    if cmd == ":w" || cmd == ":write" || cmd == ":wq" {
        return (ParsedCommand::WriteQuit, vec![]);
    }
    for prefix in &["/", ":s ", ":search "] {
        if cmd.starts_with(prefix) {
            let rest = cmd[prefix.len()..].trim().to_string();
            let args: Vec<String> = rest.split_whitespace().map(|s| s.to_string()).collect();
            return (ParsedCommand::Search, args);
        }
    }
    for prefix in &[":regex "] {
        if cmd.starts_with(prefix) {
            let rest = cmd[prefix.len()..].trim().to_string();
            let args: Vec<String> = rest.split_whitespace().map(|s| s.to_string()).collect();
            return (ParsedCommand::Regex, args);
        }
    }
    for prefix in &[":g ", ":goto "] {
        if cmd.starts_with(prefix) {
            let rest = cmd[prefix.len()..].trim().to_string();
            let args: Vec<String> = rest.split_whitespace().map(|s| s.to_string()).collect();
            return (ParsedCommand::Goto, args);
        }
    }
    for prefix in &[":w ", ":write "] {
        if cmd.starts_with(prefix) {
            let rest = cmd[prefix.len()..].trim().to_string();
            let args: Vec<String> = rest.split_whitespace().map(|s| s.to_string()).collect();
            return (ParsedCommand::Write, args);
        }
    }
    if cmd.starts_with(':') {
        let rest = cmd[1..].trim().to_string();
        let args: Vec<String> = rest.split_whitespace().map(|s| s.to_string()).collect();
        return (ParsedCommand::Goto, args);
    }
    (ParsedCommand::Unknown, vec![])
}
