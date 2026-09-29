use crate::UserEvent;
use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use tao::event_loop::EventLoopProxy;

#[derive(serde::Serialize)]
pub struct Match {
    pub name: String,
    pub path: String,
    pub folder: String,
}
#[derive(serde::Serialize)]
pub struct SearchResult {
    pub request: u64,
    pub matches: Vec<Match>,
    pub limited: bool,
    pub skipped: bool,
}
pub struct Search {
    pub request: u64,
    pub root: PathBuf,
    pub query: String,
    pub hidden: bool,
}
pub struct Operation {
    pub request: u64,
    pub root: PathBuf,
    pub path: PathBuf,
    pub name: String,
    pub rename: bool,
}
pub struct OperationResult {
    pub request: u64,
    pub result: Result<(Option<PathBuf>, PathBuf), String>,
}
pub struct Worker {
    pending: Arc<(Mutex<Option<Search>>, Condvar)>,
    generation: Arc<AtomicU64>,
    operations: mpsc::SyncSender<Operation>,
}
impl Worker {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        let pending = Arc::new((Mutex::new(None::<Search>), Condvar::new()));
        let generation = Arc::new(AtomicU64::new(0));
        let (queue, signal) = (pending.clone(), generation.clone());
        let search_proxy = proxy.clone();
        std::thread::spawn(move || loop {
            let (lock, condition) = &*queue;
            let mut waiting = lock.lock().unwrap_or_else(|e| e.into_inner());
            while waiting.is_none() {
                waiting = condition.wait(waiting).unwrap_or_else(|e| e.into_inner());
            }
            let Some(task) = waiting.take() else { continue };
            drop(waiting);
            let result = search(&task, &signal);
            if signal.load(Ordering::Relaxed) == task.request {
                let _ = search_proxy.send_event(UserEvent::QuickMatches(result));
            }
        });
        let (operations, receive) = mpsc::sync_channel::<Operation>(1);
        std::thread::spawn(move || {
            while let Ok(task) = receive.recv() {
                let result = operate(&task).map_err(|e| e.to_string());
                let _ = proxy.send_event(UserEvent::FileOperation(OperationResult {
                    request: task.request,
                    result,
                }));
            }
        });
        Self {
            pending,
            generation,
            operations,
        }
    }
    pub fn search(&self, task: Search) {
        self.generation.store(task.request, Ordering::Relaxed);
        *self.pending.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(task);
        self.pending.1.notify_one();
    }
    pub fn cancel_search(&self, request: u64) {
        self.generation.store(request, Ordering::Relaxed);
        *self.pending.0.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
    pub fn operate(&self, task: Operation) -> bool {
        self.operations.try_send(task).is_ok()
    }
}

pub fn search(task: &Search, generation: &AtomicU64) -> SearchResult {
    let query = task.query.replace('\\', "/").to_lowercase();
    let mut result = SearchResult {
        request: task.request,
        matches: Vec::new(),
        limited: false,
        skipped: false,
    };
    let mut queue = VecDeque::from([(task.root.clone(), 0u32)]);
    let mut visited = 0usize;
    let started = std::time::Instant::now();
    'folders: while let Some((folder, depth)) = queue.pop_front() {
        if generation.load(Ordering::Relaxed) != task.request {
            break;
        }
        let entries = match std::fs::read_dir(&folder) {
            Ok(entries) => entries,
            Err(_) => {
                result.skipped = true;
                continue;
            }
        };
        for entry in entries {
            if generation.load(Ordering::Relaxed) != task.request {
                break 'folders;
            }
            visited += 1;
            if visited > 20_000 || started.elapsed().as_secs() >= 2 || result.matches.len() >= 200 {
                result.limited = true;
                break 'folders;
            }
            let Ok(entry) = entry else {
                result.skipped = true;
                continue;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            if !task.hidden && name.starts_with('.') {
                continue;
            }
            let Ok(kind) = entry.file_type() else {
                result.skipped = true;
                continue;
            };
            if kind.is_symlink() {
                continue;
            }
            let path = entry.path();
            if kind.is_dir() {
                if [".git", ".hg", ".svn"].contains(&name.as_str()) {
                    continue;
                }
                if depth < 64 && queue.len() < 1024 {
                    queue.push_back((path, depth + 1));
                } else {
                    result.limited = true;
                }
                continue;
            }
            if !kind.is_file() || !crate::tree::openable(&path) {
                continue;
            }
            let relative = path.strip_prefix(&task.root).unwrap_or(&path);
            if !relative
                .to_string_lossy()
                .replace('\\', "/")
                .to_lowercase()
                .contains(&query)
            {
                continue;
            }
            result.matches.push(Match {
                name,
                path: path.to_string_lossy().into_owned(),
                folder: relative
                    .parent()
                    .unwrap_or(Path::new(""))
                    .to_string_lossy()
                    .into_owned(),
            });
        }
    }
    result.matches.sort_by_cached_key(|m| {
        (
            !m.name.to_lowercase().starts_with(&query),
            m.name.to_lowercase(),
            m.folder.to_lowercase(),
        )
    });
    if result.matches.len() > 100 {
        result.matches.truncate(100);
        result.limited = true;
    }
    result
}

pub fn valid_name(name: &str) -> bool {
    let base = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    !name.is_empty()
        && name.len() <= 255
        && name != "."
        && name != ".."
        && !name.ends_with(['.', ' '])
        && !name
            .chars()
            .any(|c| c.is_control() || "/\\<>:\"|?*".contains(c))
        && !["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].contains(&base.as_str())
        && !(base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && matches!(base.as_bytes()[3], b'1'..=b'9'))
}

pub fn operate(task: &Operation) -> io::Result<(Option<PathBuf>, PathBuf)> {
    if !task.path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Choose a file or folder from the workspace.",
        ));
    }
    if !valid_name(&task.name) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Use a single valid file or folder name, without path separators.",
        ));
    }
    let root = task.root.canonicalize()?;
    let (old, parent) = if task.rename {
        let parent = task
            .path
            .parent()
            .ok_or_else(|| io::Error::other("This location cannot be renamed."))?
            .canonicalize()?;
        let name = task
            .path
            .file_name()
            .ok_or_else(|| io::Error::other("This location cannot be renamed."))?;
        let old = parent.join(name);
        std::fs::symlink_metadata(&old)?;
        (Some(old), parent)
    } else {
        (None, task.path.canonicalize()?)
    };
    if !parent.starts_with(&root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Choose a location inside the current workspace.",
        ));
    }
    let destination = parent.join(&task.name);
    if let Some(old) = &old {
        if *old != destination {
            rename_no_replace(old, &destination)?;
        }
    } else {
        std::fs::create_dir(&destination)?;
    }
    Ok((old, destination))
}

pub fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        let from = CString::new(from.as_os_str().as_bytes())
            .map_err(|_| io::Error::other("Invalid source path"))?;
        let to = CString::new(to.as_os_str().as_bytes())
            .map_err(|_| io::Error::other("Invalid destination path"))?;
        #[cfg(target_os = "linux")]
        let result = unsafe {
            unsafe extern "C" {
                fn renameat2(
                    oldfd: i32,
                    old: *const std::ffi::c_char,
                    newfd: i32,
                    new: *const std::ffi::c_char,
                    flags: u32,
                ) -> i32;
            }
            renameat2(-100, from.as_ptr(), -100, to.as_ptr(), 1)
        };
        #[cfg(target_os = "macos")]
        let result = unsafe {
            unsafe extern "C" {
                fn renamex_np(
                    old: *const std::ffi::c_char,
                    new: *const std::ffi::c_char,
                    flags: u32,
                ) -> i32;
            }
            renamex_np(from.as_ptr(), to.as_ptr(), 4)
        };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    #[cfg(target_os = "windows")]
    {
        let from = crate::file_metadata::windows_path(from)?;
        let to = crate::file_metadata::windows_path(to)?;
        if unsafe { windows_sys::Win32::Storage::FileSystem::MoveFileW(from.as_ptr(), to.as_ptr()) }
            == 0
        {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(())
}

pub fn rebase(path: &Path, old: &Path, new: &Path) -> Option<PathBuf> {
    if let Ok(tail) = path.strip_prefix(old) {
        return Some(if tail.as_os_str().is_empty() {
            new.to_path_buf()
        } else {
            new.join(tail)
        });
    }
    #[cfg(target_os = "windows")]
    {
        fn plain(path: &Path) -> PathBuf {
            use std::path::{Component, Prefix};
            let mut parts = path.components();
            let mut out = match parts.next() {
                Some(Component::Prefix(prefix)) => match prefix.kind() {
                    Prefix::VerbatimDisk(drive) => PathBuf::from(format!("{}:\\", drive as char)),
                    Prefix::VerbatimUNC(host, share) => {
                        let mut p = PathBuf::from(r"\\");
                        p.push(host);
                        p.push(share);
                        p
                    }
                    _ => return path.to_path_buf(),
                },
                _ => return path.to_path_buf(),
            };
            for part in parts {
                if part != Component::RootDir {
                    out.push(part.as_os_str());
                }
            }
            out
        }
        let path = plain(path);
        let old = plain(old);
        let mut actual = path.components();
        for wanted in old.components() {
            let got = actual.next()?;
            if !got
                .as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case(&wanted.as_os_str().to_string_lossy())
            {
                return None;
            }
        }
        let mut result = new.to_path_buf();
        for tail in actual {
            result.push(tail.as_os_str());
        }
        Some(result)
    }
    #[cfg(not(target_os = "windows"))]
    None
}
