use std::fs::File;
use std::io::{self, Read};

use crate::config;
use crate::core::editor::{Command, LogEntry};
use crate::core::log_writer::serializer::{get_serializer, Serializer};

fn length_prefix_read(r: &mut dyn Read) -> io::Result<Vec<u8>> {
    let mut lb = [0u8; 8];
    r.read_exact(&mut lb)?;
    let len = u64::from_le_bytes(lb) as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)
        .map_err(|_| io::Error::new(io::ErrorKind::UnexpectedEof, "incomplete read"))?;
    Ok(buf)
}

pub fn read<F>(filename: &str, mut apply: F) -> io::Result<()>
where
    F: FnMut(LogEntry) -> bool,
{
    let mut file = File::open(filename)?;
    let cfg = config::load();
    let mut serializer: Box<dyn Serializer> = get_serializer(cfg.initial_serializer_version)?;

    loop {
        let b = match length_prefix_read(&mut file) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e),
        };

        let entry = serializer.unmarshal(&b)?;

        match entry.command {
            Command::SetVersion => {
                serializer = get_serializer(entry.version)?;
            }
            _ => {
                if !apply(entry) {
                    return Ok(());
                }
            }
        }
    }
}
