use crate::config;
use crate::core::editor::LogEntry;
use std::io;

pub trait Serializer: Send + Sync {
    fn marshal(&self, entry: &LogEntry) -> io::Result<Vec<u8>>;
    fn unmarshal(&self, bytes: &[u8]) -> io::Result<LogEntry>;
    fn version(&self) -> u64;
}

pub fn get_serializer(version: u64) -> io::Result<Box<dyn Serializer>> {
    match version {
        config::HUMAN_READABLE_SERIALIZER => Ok(Box::new(HumanReadableSerializer)),
        config::BINARY_SERIALIZER => Ok(Box::new(BinarySerializer)),
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "serializer not found")),
    }
}

// ---------------------------------------------------------------------------
// Human-readable (JSON) serializer
// ---------------------------------------------------------------------------

pub struct HumanReadableSerializer;

impl Serializer for HumanReadableSerializer {
    fn marshal(&self, entry: &LogEntry) -> io::Result<Vec<u8>> {
        let mut b = serde_json::to_vec(entry)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut out = vec![b' '];
        out.append(&mut b);
        out.push(b'\n');
        Ok(out)
    }

    fn unmarshal(&self, bytes: &[u8]) -> io::Result<LogEntry> {
        serde_json::from_slice(bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    fn version(&self) -> u64 {
        config::HUMAN_READABLE_SERIALIZER
    }
}

// ---------------------------------------------------------------------------
// Binary serializer (minimal — same as Go implementation)
// ---------------------------------------------------------------------------

pub struct BinarySerializer;

fn u64_to_bytes(x: u64) -> [u8; 8] {
    x.to_le_bytes()
}
fn bytes_to_u64(b: &[u8]) -> u64 {
    u64::from_le_bytes(b.try_into().unwrap())
}
fn char_to_bytes(c: char) -> [u8; 4] {
    (c as u32).to_le_bytes()
}
fn bytes_to_char(b: &[u8]) -> char {
    char::from_u32(u32::from_le_bytes(b.try_into().unwrap())).unwrap_or('\0')
}

use crate::core::editor::Command;

fn command_to_byte(c: &Command) -> u8 {
    match c {
        Command::SetVersion => 0,
        Command::Type => 1,
        Command::Enter => 2,
        Command::Backspace => 3,
        Command::Delete => 4,
        Command::Undo => 5,
        Command::Redo => 6,
        Command::InsertLine => 7,
        Command::DeleteLine => 8,
    }
}

fn byte_to_command(b: u8) -> io::Result<Command> {
    match b {
        0 => Ok(Command::SetVersion),
        1 => Ok(Command::Type),
        2 => Ok(Command::Enter),
        3 => Ok(Command::Backspace),
        4 => Ok(Command::Delete),
        5 => Ok(Command::Undo),
        6 => Ok(Command::Redo),
        7 => Ok(Command::InsertLine),
        8 => Ok(Command::DeleteLine),
        _ => Err(io::Error::new(io::ErrorKind::InvalidData, "unknown command byte")),
    }
}

impl Serializer for BinarySerializer {
    fn marshal(&self, entry: &LogEntry) -> io::Result<Vec<u8>> {
        let mut buf = vec![command_to_byte(&entry.command)];
        match entry.command {
            Command::SetVersion => {
                buf.extend_from_slice(&u64_to_bytes(entry.version));
            }
            Command::Type => {
                buf.extend_from_slice(&u64_to_bytes(entry.row));
                buf.extend_from_slice(&u64_to_bytes(entry.col));
                buf.extend_from_slice(&char_to_bytes(entry.rune));
            }
            Command::Enter | Command::Backspace | Command::Delete => {
                buf.extend_from_slice(&u64_to_bytes(entry.row));
                buf.extend_from_slice(&u64_to_bytes(entry.col));
            }
            Command::Undo | Command::Redo => {}
            _ => {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "binary: unsupported command"));
            }
        }
        Ok(buf)
    }

    fn unmarshal(&self, bytes: &[u8]) -> io::Result<LogEntry> {
        if bytes.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "empty bytes"));
        }
        let cmd = byte_to_command(bytes[0])?;
        let mut entry = LogEntry { command: cmd.clone(), ..Default::default() };
        let rest = &bytes[1..];
        match cmd {
            Command::SetVersion => {
                entry.version = bytes_to_u64(&rest[..8]);
            }
            Command::Type => {
                entry.row = bytes_to_u64(&rest[..8]);
                entry.col = bytes_to_u64(&rest[8..16]);
                entry.rune = bytes_to_char(&rest[16..20]);
            }
            Command::Enter | Command::Backspace | Command::Delete => {
                entry.row = bytes_to_u64(&rest[..8]);
                entry.col = bytes_to_u64(&rest[8..16]);
            }
            Command::Undo | Command::Redo => {}
            _ => {}
        }
        Ok(entry)
    }

    fn version(&self) -> u64 {
        config::BINARY_SERIALIZER
    }
}

