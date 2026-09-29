use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

struct State {
    directory: PathBuf,
    date: String,
    file: Option<std::fs::File>,
}

static STATE: OnceLock<Mutex<State>> = OnceLock::new();

/// Days since the epoch converted to a calendar date, so a dated log file
/// needs no date library.
fn today() -> String {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0) as i64;
    let (mut y, mut d) = (1970, days);
    loop {
        let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
        let len = if leap { 366 } else { 365 };
        if d < len {
            break;
        }
        d -= len;
        y += 1;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0;
    while d >= months[m] {
        d -= months[m];
        m += 1;
    }
    format!("{:04}-{:02}-{:02}", y, m + 1, d + 1)
}

pub fn init(root: &Path) {
    let directory = root.join("logs");
    let _ = std::fs::create_dir_all(&directory);
    let _ = STATE.set(Mutex::new(State {
        directory,
        date: String::new(),
        file: None,
    }));
}

pub fn line(msg: &str) {
    let Some(state) = STATE.get() else { return };
    let Ok(mut state) = state.try_lock() else {
        return;
    };
    let date = today();
    if state.date != date || state.file.is_none() {
        state.date = date.clone();
        let mut options = std::fs::OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        state.file = options
            .open(
                state
                    .directory
                    .join(format!("{}-{date}.log", env!("CARGO_PKG_NAME"))),
            )
            .ok();
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() % 86_400)
        .unwrap_or(0);
    if let Some(file) = &mut state.file {
        let _ = writeln!(
            file,
            "{} {:02}:{:02}:{:02}Z {}",
            date,
            stamp / 3600,
            (stamp % 3600) / 60,
            stamp % 60,
            msg
        );
    }
}
