use std::io::{self};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::log_writer::reader;
use crate::ui::util::make_insert_editor;

pub fn run_replay(input_filename: &str, log_filename: &str) -> io::Result<()> {
    eprintln!("loading input file {}", input_filename);

    let stop = Arc::new(AtomicBool::new(false));
    let (setup, finalizer) = make_insert_editor(input_filename, "", 20, 20, stop.clone())?;

    // Consume views (background progress) in a thread
    let rx = setup.rx;
    std::thread::spawn(move || {
        for view in rx {
            if !view.status.background.is_empty() {
                eprintln!("{}", view.status.background);
            }
        }
    });

    // Wait for loading to finish
    while !setup.load_done.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    eprintln!("loading log file {}", log_filename);

    let editor = setup.insert_editor;
    reader::read(log_filename, |entry| {
        editor.apply(entry);
        true
    })?;

    eprintln!("replaying file");
    let view = editor.get_view();
    let t = view.text;
    for i in 0..t.len() {
        let line: String = t.get(i).into_iter().collect();
        println!("{}", line);
    }

    finalizer.close();
    Ok(())
}
