use std::env;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

pub const VERSION: &str = "0.1.9";
pub const HUMAN_READABLE_SERIALIZER: u64 = 0;
pub const BINARY_SERIALIZER: u64 = 1;

pub const HELP: &str = r#"
Usage: "microscope [option] file [logfile]"
Options:
  -h --help           show help
  -v --version        get version
  -r --replay         replay the edited file
  -l --log            print the human readable log format
  -i --insert         open with INSERT mode
  -c --command        open with NORMAL/COMMAND/VISUAL/INSERT mode
     --unsafe         open with UNSAFE mode

Keyboard Shortcuts:
  Ctrl+C              exit
  Ctrl+S              flush log (autosave is always on, so this is not necessary)
  Ctrl+U              undo
  Ctrl+R              redo

NORMAL/COMMAND/VISUAL/INSERT mode:
  in NORMAL mode:
    i                 enter INSERT mode
    :                 enter COMMAND mode
    V                 enter VISUAL mode
    p                 paste from clipboard
  in COMMAND mode:
    ENTER             execute command
    ESCAPE            delete command buffer and enter NORMAL mode
  in INSERT mode:
    ESCAPE            enter NORMAL mode
  in VISUAL mode:
    up,dn,pgup,pgdn   move cursor and selector
    d                 cut into clipboard
    y                 copy into clipboard
    ESCAPE            enter NORMAL mode

Commands:
  :i :insert        enter INSERT mode
  / :s :search      search
  :regex            search with regex
  : :g :goto        goto line
  :w :write         write into file
  :q :quit          quit
"#;

#[derive(Clone, Debug)]
pub struct Config {
    pub debug: bool,
    pub version: &'static str,
    pub log_autoflush_interval: Duration,
    pub loading_progress_interval: Duration,
    pub serializer_version: u64,
    pub initial_serializer_version: u64,
    pub maxsize_history_stack: usize,
    pub view_channel_size: usize,
    pub max_search_time: Duration,
    pub tab_size: usize,
    pub log_dir: PathBuf,
    pub tmp_dir: PathBuf,
    pub scroll_speed: usize,
    pub load_escape_interval: Duration,
}

static CONFIG: OnceLock<Mutex<Option<Config>>> = OnceLock::new();

fn load_default_config() -> Config {
    let temp_dir = env::temp_dir();
    let log_dir = temp_dir.join("microscope").join("log");
    let tmp_dir = temp_dir.join("microscope").join("tmp");
    let debug = env::var("DEBUG").map(|v| !v.is_empty()).unwrap_or(false);
    Config {
        debug,
        version: VERSION,
        log_autoflush_interval: Duration::from_secs(60),
        loading_progress_interval: Duration::from_millis(100),
        serializer_version: HUMAN_READABLE_SERIALIZER,
        initial_serializer_version: HUMAN_READABLE_SERIALIZER,
        maxsize_history_stack: 1024,
        view_channel_size: 64,
        max_search_time: Duration::from_secs(5),
        tab_size: 2,
        log_dir,
        tmp_dir,
        scroll_speed: 3,
        load_escape_interval: Duration::from_millis(100),
    }
}

pub fn load() -> Config {
    let cell = CONFIG.get_or_init(|| Mutex::new(None));
    let mut guard = cell.lock().unwrap();
    if guard.is_none() {
        *guard = Some(load_default_config());
    }
    guard.as_ref().unwrap().clone()
}
