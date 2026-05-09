use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent,
        KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::config;
use crate::core::editor::{ActionVal, Cursor, LogEntry};
use crate::core::multimode_editor::MultiModeEditor;
use crate::ui::draw::draw;
use crate::ui::util::make_insert_editor;

pub fn run_editor(input_filename: &str, log_filename: &str, multi_mode: bool) -> io::Result<()> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag2 = stop_flag.clone();

    let (setup, finalizer) = make_insert_editor(
        input_filename,
        log_filename,
        80, // will be updated after terminal setup
        24,
        stop_flag.clone(),
    )?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let (cols, rows) = crossterm::terminal::size()?;
    let width = cols as usize;
    let height = rows as usize;

    // Resize after we know terminal size
    setup.insert_editor.resize(height.saturating_sub(1), width);

    let editor_arc = setup.insert_editor.clone();

    let multi_editor = if multi_mode {
        let stop = {
            let sf = stop_flag2.clone();
            Arc::new(move || {
                sf.store(true, Ordering::Relaxed);
            }) as Arc<dyn Fn() + Send + Sync>
        };
        Some(Arc::new(MultiModeEditor::new(
            editor_arc.clone(),
            stop,
            input_filename.to_string(),
        )))
    } else {
        None
    };

    // Draw loop thread
    let rx = setup.rx;
    let stop_draw = stop_flag2.clone();
    std::thread::spawn(move || {
        let mut stdout = io::stdout();
        for view in rx {
            if stop_draw.load(Ordering::Relaxed) {
                break;
            }
            let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
            let _ = draw(&mut stdout, &view, cols, rows);
        }
    });

    // Auto-flush timer thread
    let flush_fn = finalizer.flush.as_ref().map(|_| {
        // can't move out of finalizer.flush easily; we'll handle inline
    });

    let cfg = config::load();
    let stop_flush = stop_flag2.clone();
    // We'll do periodic flush in the event loop below.

    let mut last_flush = std::time::Instant::now();

    // Initial draw
    {
        let view = editor_arc.get_view();
        let (cols, rows) = crossterm::terminal::size()?;
        let _ = draw(&mut stdout, &view, cols, rows);
    }

    // Event loop
    loop {
        if stop_flag2.load(Ordering::Relaxed) {
            break;
        }

        // Auto-flush
        if last_flush.elapsed() >= cfg.log_autoflush_interval {
            let _ = finalizer.flush();
            last_flush = std::time::Instant::now();
        }

        if !event::poll(std::time::Duration::from_millis(50))? {
            continue;
        }

        let ev = event::read()?;

        match ev {
            Event::Key(key) => {
                // Ctrl+C always quits
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }
                // Ctrl+S flush
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
                    let _ = finalizer.flush();
                    continue;
                }

                if let Some(me) = &multi_editor {
                    handle_multi_key(me.as_ref(), key);
                } else {
                    handle_insert_key(&editor_arc, key);
                }
            }
            Event::Mouse(mouse) => {
                if let Some(me) = &multi_editor {
                    handle_mouse(me.as_ref(), mouse);
                }
            }
            Event::Resize(cols, rows) => {
                let w = cols as usize;
                let h = rows as usize;
                if let Some(me) = &multi_editor {
                    me.inner.resize(h.saturating_sub(1), w);
                } else {
                    editor_arc.resize(h.saturating_sub(1), w);
                }
            }
            _ => {}
        }

        if stop_flag2.load(Ordering::Relaxed) {
            break;
        }
    }

    // Cleanup
    let _ = finalizer.flush();
    let _ = std::fs::remove_file(log_filename);
    finalizer.close();

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;

    Ok(())
}

fn handle_insert_key(editor: &crate::core::insert_editor::InsertEditor, key: KeyEvent) {
    match key.code {
        KeyCode::Char(c) => editor.type_char(c),
        KeyCode::Enter => editor.enter(),
        KeyCode::Backspace => editor.backspace(),
        KeyCode::Delete => editor.delete(),
        KeyCode::Left => editor.move_left(),
        KeyCode::Right => editor.move_right(),
        KeyCode::Up => editor.move_up(),
        KeyCode::Down => editor.move_down(),
        KeyCode::Home => editor.move_home(),
        KeyCode::End => editor.move_end(),
        KeyCode::PageUp => editor.move_page_up(),
        KeyCode::PageDown => editor.move_page_down(),
        _ => {}
    }
}

fn handle_multi_key(editor: &MultiModeEditor, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('u') => { editor.undo(); return; }
            KeyCode::Char('r') => { editor.redo(); return; }
            _ => {}
        }
    }
    match key.code {
        KeyCode::Char(c) => editor.type_char(c),
        KeyCode::Enter => editor.enter(),
        KeyCode::Backspace => editor.backspace(),
        KeyCode::Delete => editor.delete(),
        KeyCode::Left => editor.move_left(),
        KeyCode::Right => editor.move_right(),
        KeyCode::Up => editor.move_up(),
        KeyCode::Down => editor.move_down(),
        KeyCode::Home => editor.move_home(),
        KeyCode::End => editor.move_end(),
        KeyCode::PageUp => editor.move_page_up(),
        KeyCode::PageDown => editor.move_page_down(),
        KeyCode::Esc => editor.action("key_escape", &[]),
        KeyCode::Tab => editor.action("key_tabular", &[]),
        _ => {}
    }
}

fn handle_mouse(editor: &MultiModeEditor, ev: MouseEvent) {
    match ev.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            editor.action("mouse_click_left", &[ActionVal::CursorPos(Cursor {
                row: ev.row as usize,
                col: ev.column as usize,
            })]);
        }
        MouseEventKind::ScrollUp => editor.action("mouse_scroll_up", &[]),
        MouseEventKind::ScrollDown => editor.action("mouse_scroll_down", &[]),
        _ => {}
    }
}
