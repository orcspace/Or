use crate::native::Document;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
    thread::JoinHandle,
    time::Duration,
};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: u64,
    pub title: String,
    pub path: Option<PathBuf>,
    pub version: u64,
    pub document: Document,
}
#[derive(Serialize, Deserialize)]
struct Header {
    id: u64,
    title: String,
    path: Option<PathBuf>,
    version: u64,
}
enum Action {
    Write(Box<Snapshot>),
    Clear,
}
struct Work {
    sequence: u64,
    action: Action,
}
#[derive(Default)]
struct Queue {
    pending: BTreeMap<u64, Work>,
    latest: BTreeMap<u64, u64>,
    sequence: u64,
    stop: bool,
    errors: BTreeMap<u64, String>,
}
impl Queue {
    fn push(&mut self, id: u64, action: Action) -> bool {
        let Some(sequence) = self.sequence.checked_add(1) else {
            return false;
        };
        self.sequence = sequence;
        self.latest.insert(id, sequence);
        self.pending.insert(id, Work { sequence, action });
        true
    }
    fn finish(&mut self, id: u64, work: Work, result: Result<(), String>) {
        if self.latest.get(&id) != Some(&work.sequence) {
            return;
        }
        match result {
            Ok(()) => {
                self.errors.remove(&id);
                self.latest.remove(&id);
            }
            Err(e) => {
                self.errors.insert(id, e);
                self.pending.insert(id, work);
            }
        }
    }
}
pub struct Recovery {
    queue: Arc<(Mutex<Queue>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}
fn encode(s: &Snapshot) -> Result<Vec<u8>, String> {
    let bytes = crate::native::encode(&s.document)?;
    let mut parts = crate::package::read_zip(&bytes)?;
    parts.insert(
        "recovery.json".into(),
        serde_json::to_vec(&Header {
            id: s.id,
            title: s.title.clone(),
            path: s.path.clone(),
            version: s.version,
        })
        .map_err(|e| e.to_string())?,
    );
    crate::package::write_zip(parts.into_iter().collect())
}
fn decode(bytes: &[u8]) -> Result<Snapshot, String> {
    let parts = crate::package::read_zip(bytes)?;
    let h: Header = serde_json::from_slice(
        parts
            .get("recovery.json")
            .ok_or("Нет метаданных recovery")?,
    )
    .map_err(|e| e.to_string())?;
    Ok(Snapshot {
        id: h.id,
        title: h.title,
        path: h.path,
        version: h.version,
        document: crate::native::decode(bytes)?,
    })
}
fn clear_file(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
impl Recovery {
    pub fn start() -> Self {
        Self::start_at(crate::util::app_dir().join("recovery"))
    }
    fn start_at(root: PathBuf) -> Self {
        let queue = Arc::new((Mutex::new(Queue::default()), Condvar::new()));
        let worker = queue.clone();
        let thread = std::thread::spawn(move || {
            loop {
                let (batch, stop) = {
                    let (mutex, wake) = &*worker;
                    let Ok(mut q) = mutex.lock() else { break };
                    while q.pending.is_empty() && !q.stop {
                        let Ok(next) = wake.wait(q) else { return };
                        q = next;
                    }
                    (std::mem::take(&mut q.pending), q.stop)
                };
                for (id, work) in batch {
                    let path = root.join(format!("{id}.trinezip"));
                    let result = match &work.action {
                        Action::Write(s) => {
                            encode(s).and_then(|b| crate::util::atomic_write(&path, &b, false))
                        }
                        Action::Clear => clear_file(&path)
                            .and_then(|_| clear_file(&root.join(format!("{id}.json")))),
                    };
                    if let Ok(mut q) = worker.0.lock() {
                        q.finish(id, work, result);
                    }
                }
                if stop {
                    break;
                }
                let Ok(q) = worker.0.lock() else { break };
                if !q.errors.is_empty() {
                    let Ok((_guard, _)) = worker.1.wait_timeout(q, Duration::from_millis(500))
                    else {
                        break;
                    };
                }
            }
        });
        Self {
            queue,
            thread: Some(thread),
        }
    }
    pub fn snapshot(&self, s: Snapshot) -> bool {
        if let Ok(mut q) = self.queue.0.lock() {
            let id = s.id;
            let queued = q.push(id, Action::Write(Box::new(s)));
            self.queue.1.notify_one();
            queued
        } else {
            false
        }
    }
    pub fn clear(&self, id: u64) {
        if let Ok(mut q) = self.queue.0.lock() {
            q.push(id, Action::Clear);
            self.queue.1.notify_one();
        }
    }
    pub fn error(&self) -> Option<String> {
        self.queue
            .0
            .lock()
            .ok()
            .and_then(|q| q.errors.values().next().cloned())
    }
    pub fn load() -> Vec<Snapshot> {
        std::fs::read_dir(crate::util::app_dir().join("recovery"))
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .take(100)
            .filter_map(|e| crate::util::read_file(&e.path()).ok())
            .filter_map(|b| {
                if b.starts_with(b"PK") {
                    decode(&b).ok()
                } else {
                    serde_json::from_slice::<Snapshot>(&b).ok()
                }
            })
            .filter(|s| s.document.validate().is_ok())
            .collect()
    }
}
impl Drop for Recovery {
    fn drop(&mut self) {
        if let Ok(mut q) = self.queue.0.lock() {
            q.stop = true;
            self.queue.1.notify_one();
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
pub const INTERVAL: Duration = Duration::from_millis(500);
#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Result<Snapshot, String> {
        Ok(Snapshot {
            id: 1,
            title: "test".into(),
            path: None,
            version: 2,
            document: Document::create("docs")?,
        })
    }
    #[test]
    fn clear_followed_by_snapshot_is_durable() -> Result<(), String> {
        let root =
            std::env::temp_dir().join(format!("trinedocs-recovery-test-{}", std::process::id()));
        let r = Recovery::start_at(root.clone());
        r.clear(1);
        r.snapshot(sample()?);
        drop(r);
        let s = decode(&crate::util::read_file(&root.join("1.trinezip"))?)?;
        assert_eq!(s.version, 2);
        std::fs::remove_file(root.join("1.trinezip")).map_err(|e| e.to_string())?;
        std::fs::remove_dir(root).map_err(|e| e.to_string())?;
        Ok(())
    }
    #[test]
    fn failed_old_clear_cannot_delete_new_snapshot() -> Result<(), String> {
        let mut q = Queue::default();
        q.push(1, Action::Clear);
        let work = q.pending.remove(&1).ok_or("work")?;
        q.push(1, Action::Write(Box::new(sample()?)));
        q.finish(1, work, Err("disk error".into()));
        assert!(matches!(
            q.pending.get(&1).map(|w| &w.action),
            Some(Action::Write(_))
        ));
        Ok(())
    }
}
