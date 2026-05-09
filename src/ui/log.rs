use std::io;

use crate::config;
use crate::core::log_writer::{reader, serializer::get_serializer};

pub fn run_log(log_filename: &str) -> io::Result<()> {
    let cfg = config::load();
    let ser = get_serializer(cfg.initial_serializer_version)?;

    reader::read(log_filename, |entry| {
        match ser.marshal(&entry) {
            Ok(b) => {
                let s = String::from_utf8_lossy(&b);
                println!("{}", s.trim());
                true
            }
            Err(_) => false,
        }
    })?;
    Ok(())
}
