use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

#[derive(Default)]
struct Queue {
    next: HashMap<PathBuf, String>,
    busy: bool,
    closed: bool,
}

type Shared = Arc<(Mutex<Queue>, Condvar)>;

pub struct ListWriter {
    shared: Shared,
    threaded: bool,
    failing: Arc<AtomicBool>,
}

impl ListWriter {
    pub fn spawn(failing: Arc<AtomicBool>) -> Self {
        let shared: Shared = Arc::default();
        let worker = shared.clone();
        let flag = failing.clone();
        let threaded = thread::Builder::new()
            .name("slot-lists".into())
            .spawn(move || work(&worker, &flag))
            .is_ok();
        ListWriter {
            shared,
            threaded,
            failing,
        }
    }

    pub fn save(&self, path: PathBuf, text: String) {
        if !self.threaded {
            return write(&path, &text, &self.failing);
        }
        let (lock, wake) = &*self.shared;
        let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
        q.next.insert(path, text);
        wake.notify_all();
    }

    pub fn flush(&self) {
        let (lock, wake) = &*self.shared;
        let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
        while !q.next.is_empty() || q.busy {
            q = wake.wait(q).unwrap_or_else(|e| e.into_inner());
        }
    }
}

impl Drop for ListWriter {
    fn drop(&mut self) {
        self.flush();
        let (lock, wake) = &*self.shared;
        lock.lock().unwrap_or_else(|e| e.into_inner()).closed = true;
        wake.notify_all();
    }
}

fn work(shared: &Shared, failing: &AtomicBool) {
    let (lock, wake) = &**shared;
    loop {
        let jobs: Vec<(PathBuf, String)> = {
            let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
            loop {
                if !q.next.is_empty() {
                    q.busy = true;
                    break q.next.drain().collect();
                }
                if q.closed {
                    return;
                }
                q = wake.wait(q).unwrap_or_else(|e| e.into_inner());
            }
        };
        for (path, text) in jobs {
            write(&path, &text, failing);
        }
        lock.lock().unwrap_or_else(|e| e.into_inner()).busy = false;
        wake.notify_all();
    }
}

fn write(path: &Path, text: &str, failing: &AtomicBool) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let written = slot_store::atomic_write(path, text.as_bytes());
    if let Err(e) = &written {
        eprintln!("slot: {}: {e}", path.display());
    }
    failing.store(written.is_err(), Ordering::Relaxed);
}
