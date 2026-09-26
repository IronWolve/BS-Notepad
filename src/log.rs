use std::collections::VecDeque;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const RING: usize = 500;

struct State {
    file: PathBuf,
    recent: VecDeque<String>,
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
    let dir = root.join("logs");
    let _ = std::fs::create_dir_all(&dir);
    let file = dir.join(format!("{}.log", today()));
    let _ = STATE.set(Mutex::new(State {
        file,
        recent: VecDeque::with_capacity(RING),
    }));
}

pub fn line(msg: &str) {
    let Some(state) = STATE.get() else { return };
    let Ok(mut s) = state.lock() else { return };
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() % 86_400)
        .unwrap_or(0);
    // UTC, and labelled as such: reading a log against the wrong clock wastes
    // more time than the offset saves.
    let entry = format!(
        "{:02}:{:02}:{:02}Z {}",
        stamp / 3600,
        (stamp % 3600) / 60,
        stamp % 60,
        msg
    );
    if s.recent.len() == RING {
        s.recent.pop_front();
    }
    s.recent.push_back(entry.clone());
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&s.file)
    {
        let _ = writeln!(f, "{}", entry);
    }
}
