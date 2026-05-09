use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::{self, Write};

use crate::core::editor::{OtherValue, Selector, View};
use crate::core::multimode_editor::{MODE_COMMAND, MODE_INSERT, MODE_NORMAL, MODE_SELECT};

fn get_mode_and_command(view: &View) -> (String, String) {
    if let Some(other) = &view.status.other {
        let mode = match other.get("mode") {
            Some(OtherValue::Str(s)) => s.clone(),
            _ => String::new(),
        };
        let command = match other.get("command") {
            Some(OtherValue::Str(s)) => s.clone(),
            _ => String::new(),
        };
        (mode, command)
    } else {
        (String::new(), String::new())
    }
}

fn get_selector(view: &View) -> Option<Selector> {
    if let Some(other) = &view.status.other {
        if let Some(OtherValue::Selector(s)) = other.get("selector") {
            return Some(s.clone());
        }
    }
    None
}

fn mode_bg_color(mode: &str) -> Color {
    match mode {
        MODE_NORMAL => Color::White,
        MODE_INSERT => Color::Yellow,
        MODE_SELECT => Color::Green,
        MODE_COMMAND => Color::Blue,
        _ => Color::White,
    }
}

pub fn draw<W: Write>(out: &mut W, view: &View, screen_width: u16, screen_height: u16) -> io::Result<()> {
    let (mode, command) = get_mode_and_command(view);
    let selector = get_selector(view);

    execute!(out, Hide, MoveTo(0, 0), Clear(ClearType::All))?;

    let content_height = screen_height.saturating_sub(1) as usize;
    let t = &view.text;

    // Draw content area
    for rel_row in 0..content_height {
        let row = view.window.tl_row + rel_row;
        let is_selected = selector.as_ref().map(|s| {
            let (beg, end) = s.interval();
            beg <= row && row <= end
        }).unwrap_or(false);

        execute!(out, MoveTo(0, rel_row as u16))?;

        if is_selected {
            execute!(out, SetBackgroundColor(Color::DarkGrey), SetForegroundColor(Color::White))?;
        }

        for rel_col in 0..screen_width as usize {
            let col = view.window.tl_col + rel_col;
            let ch = if row < t.len() {
                let line = t.get(row);
                if col < line.len() {
                    line[col]
                } else if rel_col == 0 {
                    ' '
                } else {
                    ' '
                }
            } else if rel_col == 0 {
                '~'
            } else {
                ' '
            };
            execute!(out, Print(ch))?;
        }

        if is_selected {
            execute!(out, ResetColor)?;
        }
    }

    // Draw status bar (last row)
    execute!(
        out,
        MoveTo(0, screen_height - 1),
        SetBackgroundColor(mode_bg_color(&mode)),
        SetForegroundColor(Color::Black)
    )?;

    // Fill the entire status bar with spaces first
    let blank: String = " ".repeat(screen_width as usize);
    execute!(out, Print(&blank))?;

    // Right side: background status
    let right_text = if !view.status.background.is_empty() {
        format!(" > {} ", view.status.background)
    } else {
        String::new()
    };

    // Left side: mode (row, col) > command > message
    let mut left = format!(" {} ({}, {})", mode, view.cursor.row + 1, view.cursor.col + 1);
    if !command.is_empty() {
        left.push_str(&format!(" > {}", command));
    }
    if !view.status.message.is_empty() {
        left.push_str(&format!(" > {}", view.status.message));
    }

    // Draw left
    execute!(out, MoveTo(0, screen_height - 1))?;
    let left_capped: String = left.chars().take(screen_width as usize).collect();
    execute!(out, Print(left_capped))?;

    // Draw right (from right edge)
    if !right_text.is_empty() {
        let right_len = right_text.chars().count();
        let right_col = screen_width as usize - right_len.min(screen_width as usize);
        execute!(out, MoveTo(right_col as u16, screen_height - 1))?;
        execute!(out, Print(&right_text))?;
    }

    execute!(out, ResetColor)?;

    // Place cursor
    let cur_col = view.cursor.col.saturating_sub(view.window.tl_col) as u16;
    let cur_row = view.cursor.row.saturating_sub(view.window.tl_row) as u16;
    execute!(out, Show, MoveTo(cur_col, cur_row))?;

    out.flush()?;
    Ok(())
}
