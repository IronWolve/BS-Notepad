use crate::{render, settings::Settings, theme::Theme, UserEvent};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Condvar, Mutex,
    },
};
use tao::event_loop::EventLoopProxy;
#[derive(Clone)]
pub struct RenderTask {
    pub generation: u64,
    pub tab: u64,
    pub revision: u64,
    pub path: Option<PathBuf>,
    pub text: String,
    pub settings: Settings,
    pub theme: Theme,
}
pub struct RenderWorker {
    pending: Arc<(Mutex<Option<RenderTask>>, Condvar)>,
    latest: Arc<AtomicU64>,
}
impl RenderWorker {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        let pending = Arc::new((Mutex::new(None::<RenderTask>), Condvar::new()));
        let inbox = pending.clone();
        let latest = Arc::new(AtomicU64::new(0));
        let active = latest.clone();
        std::thread::spawn(move || {
            let mut renderer = render::Renderer::new();
            let mut cache: VecDeque<(String, render::Document)> = VecDeque::new();
            let mut bytes = 0;
            loop {
                let task = {
                    let (lock, wake) = &*inbox;
                    let mut slot = lock.lock().unwrap();
                    while slot.is_none() {
                        slot = wake.wait(slot).unwrap();
                    }
                    slot.take().unwrap()
                };
                if task.generation != active.load(Ordering::Relaxed) {
                    continue;
                }
                renderer.cancel_token(active.clone(), task.generation);
                let key = format!(
                    "{:?}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
                    task.path,
                    task.tab,
                    task.revision,
                    task.theme.id,
                    task.theme.text_contrast,
                    task.settings.view_mode,
                    task.settings.syntax_colour,
                    task.settings.highlight_limit_kb,
                    task.settings.plain_text_above_mb,
                    task.settings.remote_images,
                    crate::storage::fingerprint(task.text.as_bytes())
                );
                let document = cache
                    .iter()
                    .find(|(k, _)| k == &key)
                    .map(|(_, doc)| doc.clone())
                    .unwrap_or_else(|| {
                        renderer.render(
                            task.path.as_deref(),
                            &task.text,
                            &task.settings,
                            &task.theme,
                        )
                    });
                if task.generation != active.load(Ordering::Relaxed) {
                    continue;
                }
                if document.html.len() < 4 * 1024 * 1024 && !cache.iter().any(|(k, _)| k == &key) {
                    bytes += document.html.len();
                    cache.push_back((key.clone(), document.clone()));
                    while bytes > 8 * 1024 * 1024 || cache.len() > 6 {
                        if let Some((_, old)) = cache.pop_front() {
                            bytes -= old.html.len();
                        }
                    }
                }
                let _ = proxy.send_event(UserEvent::Rendered {
                    generation: task.generation,
                    tab: task.tab,
                    revision: task.revision,
                    key,
                    document,
                });
            }
        });
        Self { pending, latest }
    }
    pub fn cancel(&self) -> u64 {
        self.latest.fetch_add(1, Ordering::Relaxed) + 1
    }
    pub fn submit(&self, mut task: RenderTask) -> u64 {
        task.generation = self.latest.fetch_add(1, Ordering::Relaxed) + 1;
        let generation = task.generation;
        let (lock, wake) = &*self.pending;
        *lock.lock().unwrap() = Some(task);
        wake.notify_one();
        generation
    }
}
#[derive(Clone)]
pub struct OpenTask {
    pub restore: bool,
    pub view: Option<crate::settings::SessionTab>,
    pub path: PathBuf,
    pub new_tab: bool,
    pub from: u64,
    pub revision: u64,
    pub reload: bool,
    pub fragment: String,
    pub limit: u32,
}
pub enum IoTask {
    #[cfg(test)]
    BlockForTest {
        tree: bool,
        entered: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    },
    Flush(mpsc::Sender<()>),
    Open(OpenTask),
    Tree {
        path: PathBuf,
        hidden: bool,
        dates: bool,
        sort_date: bool,
        request: serde_json::Value,
        root: bool,
        serial: u64,
    },
    Check {
        tab: u64,
        path: PathBuf,
        epoch: u64,
        force: bool,
    },
    Recovery {
        root: PathBuf,
        key: String,
        document: Option<crate::recovery::Draft>,
    },
}
#[derive(Default)]
struct RecoveryQueue {
    pending: std::collections::HashMap<(PathBuf, String), Option<crate::recovery::Draft>>,
    barriers: Vec<mpsc::Sender<()>>,
    closed: bool,
}
type EventSink = Arc<dyn Fn(UserEvent) + Send + Sync>;
pub struct IoWorker {
    files: mpsc::Sender<IoTask>,
    trees: mpsc::Sender<IoTask>,
    checks: mpsc::Sender<IoTask>,
    recovery: Arc<(Mutex<RecoveryQueue>, Condvar)>,
}
impl IoWorker {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        Self::with_sink(Arc::new(move |event| {
            let _ = proxy.send_event(event);
        }))
    }
    fn with_sink(events: EventSink) -> Self {
        fn lane(events: EventSink) -> mpsc::Sender<IoTask> {
            let (sender, receiver) = mpsc::channel();
            std::thread::spawn(move || {
                let mut monitor = crate::disk::Monitor::default();
                while let Ok(task) = receiver.recv() {
                    match task {
                        IoTask::Open(task) => {
                            let path = crate::paths::normalize(&task.path);
                            let result = if path.to_str().is_none() {
                                Err("This filename contains characters that cannot be represented in the interface. Rename it using your file manager first.".into())
                            } else {
                                crate::storage::read(&path, task.limit).map_err(|e| e.to_string())
                            };
                            events(UserEvent::Loaded {
                                task: OpenTask { path, ..task },
                                result,
                            });
                        }
                        IoTask::Tree {
                            path,
                            hidden,
                            dates,
                            sort_date,
                            request,
                            root,
                            serial,
                        } => {
                            let path = crate::paths::normalize(&path);
                            let result =
                                crate::tree::list_with_options(&path, hidden, dates, sort_date)
                                    .map_err(|e| e.to_string());
                            events(UserEvent::TreeLoaded {
                                path,
                                request,
                                root,
                                serial,
                                result,
                            });
                        }
                        IoTask::Check {
                            tab,
                            path,
                            epoch,
                            force,
                        } => {
                            let observed = monitor.check(&path, force);
                            events(UserEvent::DiskChecked {
                                tab,
                                path,
                                epoch,
                                observed,
                            });
                        }
                        #[cfg(test)]
                        IoTask::BlockForTest {
                            entered, release, ..
                        } => {
                            let _ = entered.send(());
                            let _ = release.recv();
                        }
                        IoTask::Recovery { .. } | IoTask::Flush(_) => {
                            unreachable!("Recovery uses its own lane")
                        }
                    }
                }
            });
            sender
        }
        let recovery = Arc::new((Mutex::new(RecoveryQueue::default()), Condvar::new()));
        let queue = recovery.clone();
        let recovery_events = events.clone();
        std::thread::spawn(move || {
            let mut reported_error = None;
            loop {
                let (lock, wake) = &*queue;
                let mut waiting = lock.lock().unwrap_or_else(|e| e.into_inner());
                while waiting.pending.is_empty() && waiting.barriers.is_empty() && !waiting.closed {
                    waiting = wake.wait(waiting).unwrap_or_else(|e| e.into_inner());
                }
                if waiting.closed && waiting.pending.is_empty() {
                    break;
                }
                let work = std::mem::take(&mut waiting.pending);
                let barriers = std::mem::take(&mut waiting.barriers);
                drop(waiting);
                let had_work = !work.is_empty();
                let mut error = None;
                for ((root, key), document) in work {
                    if let Err(failed) = crate::recovery::write(&root, &key, document.as_ref()) {
                        error = Some(failed.to_string());
                    }
                }
                if had_work && error.is_some() != reported_error.is_some() {
                    recovery_events(UserEvent::RecoveryStatus(error.clone()));
                }
                if had_work {
                    reported_error = error;
                }
                // Acknowledge queue completion; write errors are separately reported.
                for barrier in barriers {
                    let _ = barrier.send(());
                }
            }
        });
        Self {
            files: lane(events.clone()),
            trees: lane(events.clone()),
            checks: lane(events),
            recovery,
        }
    }
    pub fn flush(&self) -> bool {
        let (tx, rx) = mpsc::channel();
        self.send(IoTask::Flush(tx));
        rx.recv_timeout(std::time::Duration::from_secs(3)).is_ok()
    }
    pub fn send(&self, task: IoTask) {
        match task {
            IoTask::Recovery {
                root,
                key,
                document,
            } => {
                self.recovery
                    .0
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .pending
                    .insert((root, key), document);
                self.recovery.1.notify_one();
            }
            IoTask::Flush(done) => {
                self.recovery
                    .0
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .barriers
                    .push(done);
                self.recovery.1.notify_one();
            }
            task @ IoTask::Tree { .. } => {
                let _ = self.trees.send(task);
            }
            task @ IoTask::Check { .. } => {
                let _ = self.checks.send(task);
            }
            #[cfg(test)]
            task @ IoTask::BlockForTest { tree: true, .. } => {
                let _ = self.trees.send(task);
            }
            task => {
                let _ = self.files.send(task);
            }
        }
    }
}
impl Drop for IoWorker {
    fn drop(&mut self) {
        self.recovery
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .closed = true;
        self.recovery.1.notify_one();
    }
}

#[cfg(test)]
mod isolation_tests {
    use super::*;
    #[test]
    fn blocked_file_and_tree_reads_do_not_block_recovery_flush() {
        let worker = IoWorker::with_sink(Arc::new(|_| {}));
        let mut releases = Vec::new();
        for tree in [false, true] {
            let (entered, started) = mpsc::channel();
            let (release, blocked) = mpsc::channel();
            worker.send(IoTask::BlockForTest {
                tree,
                entered,
                release: blocked,
            });
            started
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap();
            releases.push(release);
        }
        assert!(worker.flush());
        for release in releases {
            release.send(()).unwrap();
        }
    }
}
