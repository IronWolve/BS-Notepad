use std::{io, path::PathBuf, sync::mpsc, time::SystemTime};
use tao::event_loop::EventLoopProxy;

pub struct Task {
    pub token: u64,
    pub tab: u64,
    pub path: PathBuf,
    pub original: Option<PathBuf>,
    pub other_paths: Vec<PathBuf>,
    pub source: String,
    pub unchanged: bool,
    pub format: crate::storage::TextFormat,
    pub baseline: Option<u64>,
    pub approved_disk: Option<Option<u64>>,
    pub approved_endings: bool,
}
pub enum Outcome {
    Conflict(Option<u64>),
    MixedEndings,
    Written {
        fingerprint: u64,
        modified: Option<SystemTime>,
        mixed: bool,
        warning: Option<String>,
    },
}
pub fn observe(path: &std::path::Path) -> io::Result<Option<u64>> {
    match crate::storage::disk_fingerprint(path) {
        Ok(hash) => Ok(Some(hash)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}
pub(crate) fn execute(task: &mut Task) -> io::Result<Outcome> {
    task.path = crate::paths::normalize(&task.path);
    if task
        .other_paths
        .iter()
        .any(|path| crate::paths::same(path, &task.path))
    {
        return Err(io::Error::other(
            "That file is open in another tab. Save from that tab, or choose another filename.",
        ));
    }
    let same_path = task
        .original
        .as_deref()
        .is_some_and(|path| crate::paths::same(path, &task.path));
    let observed = observe(&task.path)?;
    let expected = task.approved_disk.unwrap_or(task.baseline);
    if (same_path && task.baseline.is_some() || task.approved_disk.is_some())
        && observed != expected
    {
        return Ok(Outcome::Conflict(observed));
    }
    if let Some(fingerprint) =
        observed.filter(|_| same_path && task.unchanged && observed == task.baseline)
    {
        return Ok(Outcome::Written {
            fingerprint,
            modified: std::fs::metadata(&task.path)
                .and_then(|m| m.modified())
                .ok(),
            mixed: task.format.mixed,
            warning: Some("Saved; file content is unchanged.".into()),
        });
    }
    if task.format.mixed && !task.approved_endings {
        task.approved_disk = Some(observed);
        return Ok(Outcome::MixedEndings);
    }
    let bytes = task.format.encode(&task.source);
    match crate::storage::write_checked_document(&task.path, &bytes, observed) {
        Ok(report) => Ok(Outcome::Written {
            fingerprint: crate::storage::fingerprint(&bytes),
            modified: std::fs::metadata(&task.path)
                .and_then(|m| m.modified())
                .ok(),
            mixed: false,
            warning: report.warning,
        }),
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
            Ok(Outcome::Conflict(observe(&task.path)?))
        }
        Err(error) => Err(error),
    }
}
pub struct Worker(mpsc::SyncSender<Task>);
impl Worker {
    pub fn new(proxy: EventLoopProxy<crate::UserEvent>) -> Self {
        let (send, receive) = mpsc::sync_channel::<Task>(1);
        std::thread::spawn(move || {
            while let Ok(mut task) = receive.recv() {
                let result = execute(&mut task).map_err(|error| error.to_string());
                let _ = proxy.send_event(crate::UserEvent::Saved { task, result });
            }
        });
        Self(send)
    }
    pub fn submit(&self, task: Task) -> Result<(), ()> {
        self.0.try_send(task).map_err(|_| ())
    }
}
