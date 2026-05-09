use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

static SIDE_CHANNEL: Mutex<SideChannel> = Mutex::new(SideChannel {
    path: ".side_channel.log",
    written: false,
});

struct SideChannel {
    path: &'static str,
    written: bool,
}

fn write_line(msg: &str) {
    let mut sc = SIDE_CHANNEL.lock().unwrap();
    if !sc.written {
        sc.written = true;
        let _ = std::fs::remove_file(sc.path);
    }
    if let Ok(mut f) = OpenOptions::new().append(true).create(true).open(sc.path) {
        let _ = writeln!(f, "{}", msg);
    }
}

pub fn write_ln(msg: &str) {
    write_line(msg);
}

pub fn panic_with(msg: &str) -> ! {
    write_line(msg);
    std::process::exit(1);
}

#[macro_export]
macro_rules! side_panic {
    ($($arg:tt)*) => {
        $crate::util::side_channel::panic_with(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! side_log {
    ($($arg:tt)*) => {
        $crate::util::side_channel::write_ln(&format!($($arg)*))
    };
}
