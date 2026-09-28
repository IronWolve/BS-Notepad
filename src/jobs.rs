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
                    "{:?}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
                    task.path,
                    task.tab,
                    task.revision,
                    task.theme.id,
                    task.theme.text_contrast,
                    task.settings.view_mode,
                    task.settings.syntax_colour,
                    task.settings.highlight_limit_kb,
                    task.settings.plain_text_above_mb,
                    task.settings.remote_images
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
    Flush(mpsc::Sender<()>),
    Open(OpenTask),
    Tree {
        path: PathBuf,
        hidden: bool,
        request: serde_json::Value,
        root: bool,
        serial: u64,
    },
    Check {
        tab: u64,
        path: PathBuf,
        fingerprint: u64,
        revision: u64,
    },
    Recovery {
        root: PathBuf,
        key: String,
        document: Option<crate::recovery::Draft>,
    },
}
pub struct IoWorker(mpsc::Sender<IoTask>);
impl IoWorker {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(task) = receiver.recv() {
                match task {
                    IoTask::Flush(done) => {
                        let _ = done.send(());
                    }
                    IoTask::Open(task) => {
                        let path = task.path.canonicalize().unwrap_or(task.path.clone());
                        let result =
                            crate::storage::read(&path, task.limit).map_err(|e| e.to_string());
                        let _ = proxy.send_event(UserEvent::Loaded {
                            task: OpenTask { path, ..task },
                            result,
                        });
                    }
                    IoTask::Tree {
                        path,
                        hidden,
                        request,
                        root,
                        serial,
                    } => {
                        let result = crate::tree::list(&path, hidden).map_err(|e| e.to_string());
                        let _ = proxy.send_event(UserEvent::TreeLoaded {
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
                        fingerprint,
                        revision,
                    } => {
                        let changed =
                            crate::storage::disk_fingerprint(&path).ok() != Some(fingerprint);
                        let _ = proxy.send_event(UserEvent::DiskChecked {
                            tab,
                            revision,
                            changed,
                        });
                    }
                    IoTask::Recovery {
                        root,
                        key,
                        document,
                    } => {
                        if let Err(error) = crate::recovery::write(&root, &key, document.as_ref()) {
                            let _ = proxy.send_event(UserEvent::RecoveryError(error.to_string()));
                        }
                    }
                }
            }
        });
        Self(sender)
    }
    pub fn flush(&self) -> bool {
        let (tx, rx) = mpsc::channel();
        self.send(IoTask::Flush(tx));
        rx.recv_timeout(std::time::Duration::from_secs(3)).is_ok()
    }
    pub fn send(&self, task: IoTask) {
        let _ = self.0.send(task);
    }
}
