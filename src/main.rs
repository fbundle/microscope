mod config;
mod core;
mod ui;
mod util;

use std::path::PathBuf;
use std::io;

fn print_help() {
    println!("microscope version {}", config::VERSION);
    println!("{}", config::HELP);
}

fn print_version() {
    println!("microscope version {}", config::VERSION);
}

struct ProgramArgs {
    option: String,
    first_filename: String,
    second_filename: String,
}

fn get_default_log_filename(input_filename: &str) -> (String, String) {
    let cfg = config::load();
    if util::file_util::non_empty(input_filename) {
        let abs = std::fs::canonicalize(input_filename)
            .unwrap_or_else(|_| PathBuf::from(input_filename));
        let first = abs.to_string_lossy().to_string();
        let log_path = cfg.log_dir.join(
            abs.to_string_lossy()
                .trim_start_matches('/')
                .to_string()
        );
        let second = log_path.to_string_lossy().to_string();
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        (first, second)
    } else {
        let log_path = cfg.log_dir.join("empty_file");
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        (String::new(), log_path.to_string_lossy().to_string())
    }
}

fn prompt_yes_no(prompt: &str, default_option: bool) -> bool {
    use std::io::{BufRead, Write};
    let stdin = io::stdin();
    let suffix = if default_option { " [Y/n]: " } else { " [y/N]: " };
    loop {
        print!("{}{}", prompt, suffix);
        io::stdout().flush().ok();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() {
            return false;
        }
        let input = line.trim().to_lowercase();
        if input.is_empty() {
            return default_option;
        }
        match input.as_str() {
            "y" | "yes" => return true,
            "n" | "no" => return false,
            _ => return false,
        }
    }
}

fn prompt_delete_log_file(args: &ProgramArgs) -> bool {
    if util::file_util::non_empty(&args.second_filename) {
        let ok = prompt_yes_no(
            &format!("log file exists ({}), delete it?", args.second_filename),
            false,
        );
        if !ok {
            return false;
        }
        if let Err(e) = std::fs::remove_file(&args.second_filename) {
            eprintln!("failed to remove log file: {}", e);
            return false;
        }
    }
    true
}

fn get_program_args() -> ProgramArgs {
    let mut args: Vec<String> = std::env::args().skip(1).collect();

    let mut option = String::new();
    if args.first().map(|s| s.starts_with('-')).unwrap_or(false) {
        option = args.remove(0);
    }

    let first_filename = if !args.is_empty() { args.remove(0) } else { String::new() };
    let second_filename = if !args.is_empty() { args.remove(0) } else { String::new() };

    if second_filename.is_empty() {
        let (first, second) = get_default_log_filename(&first_filename);
        ProgramArgs { option, first_filename: first, second_filename: second }
    } else {
        let (first, _) = get_default_log_filename(&first_filename);
        ProgramArgs { option, first_filename: first, second_filename }
    }
}

fn main() {
    let args = get_program_args();

    let result = match args.option.as_str() {
        "-h" | "--help" => {
            print_help();
            Ok(())
        }
        "-v" | "--version" => {
            print_version();
            Ok(())
        }
        "-r" | "--replay" => {
            ui::replay::run_replay(&args.first_filename, &args.second_filename)
        }
        "-l" | "--log" => {
            ui::log::run_log(&args.first_filename)
        }
        "-i" | "--insert" => {
            if !prompt_delete_log_file(&args) {
                return;
            }
            ui::editor::run_editor(&args.first_filename, &args.second_filename, false)
        }
        "--unsafe" => {
            ui::editor::run_editor(&args.first_filename, "", true)
        }
        _ => {
            // default: open with command mode
            if !prompt_delete_log_file(&args) {
                return;
            }
            ui::editor::run_editor(&args.first_filename, &args.second_filename, true)
        }
    };

    if let Err(e) = result {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}
