use crate::{settings::Settings, UserEvent};
use std::{
    path::PathBuf,
    sync::{mpsc, Arc, Condvar, Mutex},
    time::{Duration, Instant},
};
use tao::event_loop::EventLoopProxy;

#[derive(Default)]
struct Queue {
    pending: Option<(u64, Settings, Instant)>,
    barriers: Vec<mpsc::Sender<bool>>,
    closed: bool,
}
pub struct Writer(Arc<(Mutex<Queue>, Condvar)>);
impl Writer {
    pub fn new(root: PathBuf, proxy: EventLoopProxy<UserEvent>) -> Self {
        let queue = Arc::new((Mutex::new(Queue::default()), Condvar::new()));
        let worker = queue.clone();
        std::thread::spawn(move || {
            let mut last_bytes = Vec::new();
            let mut last_ok = true;
            loop {
                let (lock, wake) = &*worker;
                let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
                while q.pending.is_none() && q.barriers.is_empty() && !q.closed {
                    q = wake.wait(q).unwrap_or_else(|e| e.into_inner());
                }
                if q.closed && q.pending.is_none() {
                    break;
                }
                if q.barriers.is_empty() && !q.closed {
                    if let Some((_, _, submitted)) = &q.pending {
                        let remaining =
                            Duration::from_millis(300).saturating_sub(submitted.elapsed());
                        if !remaining.is_zero() {
                            drop(
                                wake.wait_timeout(q, remaining)
                                    .unwrap_or_else(|e| e.into_inner()),
                            );
                            continue;
                        }
                    }
                }
                let task = q.pending.take();
                let barriers = std::mem::take(&mut q.barriers);
                drop(q);
                if let Some((serial, settings, _)) = task {
                    let bytes = serde_json::to_vec(&settings).unwrap_or_default();
                    let result = if bytes == last_bytes && last_ok {
                        Ok(())
                    } else {
                        settings.save(&root)
                    };
                    last_ok = result.is_ok();
                    if last_ok {
                        last_bytes = bytes;
                    }
                    let _ = proxy.send_event(UserEvent::PreferencesSaved {
                        serial,
                        error: result.err().map(|e| e.to_string()),
                    });
                }
                for done in barriers {
                    let _ = done.send(last_ok);
                }
            }
        });
        Self(queue)
    }
    pub fn submit(&self, serial: u64, settings: Settings) {
        self.0 .0.lock().unwrap_or_else(|e| e.into_inner()).pending =
            Some((serial, settings, Instant::now()));
        self.0 .1.notify_one();
    }
    pub fn flush(&self) -> bool {
        let (send, receive) = mpsc::channel();
        self.0
             .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .barriers
            .push(send);
        self.0 .1.notify_one();
        receive
            .recv_timeout(Duration::from_secs(3))
            .unwrap_or(false)
    }
}
impl Drop for Writer {
    fn drop(&mut self) {
        self.0 .0.lock().unwrap_or_else(|e| e.into_inner()).closed = true;
        self.0 .1.notify_one();
    }
}
