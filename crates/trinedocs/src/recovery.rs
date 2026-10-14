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
    Restore {
        previous_id: u64,
        snapshot: Box<Snapshot>,
    },
    Clear,
    ClearRestored {
        previous_id: u64,
    },
}
struct Work {
    sequence: u64,
    action: Action,
}
#[derive(Default)]
struct Queue {
    pending: BTreeMap<u64, Work>,
    latest: BTreeMap<u64, u64>,
    handoffs: BTreeMap<u64, u64>,
    sequence: u64,
    stop: bool,
    errors: BTreeMap<u64, String>,
}
impl Queue {
    fn push(&mut self, id: u64, mut action: Action) -> bool {
        // Preserve handoffs for queued and in-flight work when later actions coalesce.
        if let Action::Write(snapshot) = action {
            action = match self.handoffs.get(&id) {
                Some(previous_id) => Action::Restore {
                    previous_id: *previous_id,
                    snapshot,
                },
                None => Action::Write(snapshot),
            };
        }
        if matches!(action, Action::Clear)
            && let Some(previous_id) = self.handoffs.get(&id)
        {
            action = Action::ClearRestored {
                previous_id: *previous_id,
            };
        }
        let Some(sequence) = self.sequence.checked_add(1) else {
            return false;
        };
        self.sequence = sequence;
        if let Action::Restore { previous_id, .. } | Action::ClearRestored { previous_id } = &action
        {
            self.handoffs.insert(id, *previous_id);
        }
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
                self.handoffs.remove(&id);
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
fn clear_snapshot(root: &Path, id: u64) -> Result<(), String> {
    clear_file(&root.join(format!("{id}.trinezip")))
        .and_then(|_| clear_file(&root.join(format!("{id}.json"))))
}
fn load_at(root: &Path) -> Vec<Snapshot> {
    let mut candidates = BTreeMap::<u64, Vec<PathBuf>>::new();
    for entry in std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }
        let path = entry.path();
        if !matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("trinezip" | "json")
        ) {
            continue;
        }
        if let Some(id) = path
            .file_stem()
            .and_then(|name| name.to_str())
            .and_then(|name| name.parse::<u64>().ok())
            && id != 0
            && id != u64::MAX
        {
            candidates.entry(id).or_default().push(path);
        }
    }
    let mut snapshots = Vec::new();
    // Newer tab IDs first; stale/temp files never consume the 100-document budget.
    for (id, paths) in candidates.into_iter().rev() {
        let newest = paths
            .into_iter()
            .filter_map(|path| {
                let bytes = crate::util::read_file(&path).ok()?;
                let zipped = path.extension().is_some_and(|ext| ext == "trinezip");
                let snapshot = if zipped {
                    decode(&bytes).ok()?
                } else {
                    serde_json::from_slice::<Snapshot>(&bytes).ok()?
                };
                (snapshot.id == id && snapshot.document.validate().is_ok())
                    .then_some((snapshot, zipped))
            })
            .max_by_key(|(snapshot, zipped)| (snapshot.version, *zipped));
        if let Some((snapshot, _)) = newest {
            snapshots.push(snapshot);
            if snapshots.len() == 100 {
                break;
            }
        }
    }
    snapshots
}
impl Recovery {
    pub fn start() -> Self {
        Self::start_at(crate::util::app_dir().join("recovery"))
    }
    #[cfg(test)]
    pub(crate) fn start_for_test(root: PathBuf) -> Self {
        Self::start_at(root)
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
                        Action::Restore {
                            previous_id,
                            snapshot,
                        } => encode(snapshot)
                            .and_then(|bytes| crate::util::atomic_write(&path, &bytes, false))
                            .and_then(|_| clear_snapshot(&root, *previous_id)),
                        Action::Clear => clear_snapshot(&root, id),
                        Action::ClearRestored { previous_id } => clear_snapshot(&root, id)
                            .and_then(|_| clear_snapshot(&root, *previous_id)),
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
    /// Queues a durable handoff: the source is removed only after the restored tab is saved.
    pub fn restore(&self, previous_id: u64, snapshot: Snapshot) -> bool {
        if previous_id == snapshot.id || previous_id == 0 || snapshot.id == 0 {
            return false;
        }
        if let Ok(mut queue) = self.queue.0.lock() {
            let accepted = queue.push(
                snapshot.id,
                Action::Restore {
                    previous_id,
                    snapshot: Box::new(snapshot),
                },
            );
            self.queue.1.notify_one();
            accepted
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
        load_at(&crate::util::app_dir().join("recovery"))
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
    fn directory() -> Result<PathBuf, String> {
        let name = format!(
            "trinedocs-recovery-deep-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos()
        );
        let root = std::env::temp_dir().join(name);
        std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        Ok(root)
    }
    #[test]
    fn restoration_handoff_is_durable_before_removing_source() -> Result<(), String> {
        let root = directory()?;
        let source = sample()?;
        crate::util::atomic_write(&root.join("1.trinezip"), &encode(&source)?, false)?;
        let mut restored = source.clone();
        restored.id = 2;
        let recovery = Recovery::start_at(root.clone());
        assert!(recovery.restore(1, restored));
        drop(recovery);
        assert!(!root.join("1.trinezip").exists());
        assert_eq!(
            decode(&crate::util::read_file(&root.join("2.trinezip"))?)?.id,
            2
        );
        std::fs::remove_dir_all(root).map_err(|e| e.to_string())?;
        Ok(())
    }
    #[test]
    fn failed_restoration_keeps_the_original_durable_snapshot() -> Result<(), String> {
        let root = directory()?;
        let source = sample()?;
        crate::util::atomic_write(&root.join("1.trinezip"), &encode(&source)?, false)?;
        let mut restored = source;
        restored.id = 2;
        if let Document::Docs(doc) = &mut restored.document {
            doc.paragraphs.clear();
        }
        let recovery = Recovery::start_at(root.clone());
        assert!(recovery.restore(1, restored));
        drop(recovery);
        assert!(root.join("1.trinezip").exists());
        assert!(!root.join("2.trinezip").exists());
        assert_eq!(load_at(&root).len(), 1);
        std::fs::remove_dir_all(root).map_err(|e| e.to_string())?;
        Ok(())
    }
    #[test]
    fn newer_coalesced_snapshot_keeps_queued_restoration_handoff() -> Result<(), String> {
        let mut queue = Queue::default();
        let mut snapshot = sample()?;
        snapshot.id = 2;
        assert!(queue.push(
            2,
            Action::Restore {
                previous_id: 1,
                snapshot: Box::new(snapshot.clone())
            }
        ));
        snapshot.version = 3;
        assert!(queue.push(2, Action::Write(Box::new(snapshot))));
        assert!(
            matches!(queue.pending.get(&2).map(|work| &work.action), Some(Action::Restore { previous_id: 1, snapshot }) if snapshot.version == 3)
        );
        Ok(())
    }
    #[test]
    fn clearing_a_queued_restoration_also_clears_its_source() -> Result<(), String> {
        let mut queue = Queue::default();
        let mut snapshot = sample()?;
        snapshot.id = 2;
        assert!(queue.push(
            2,
            Action::Restore {
                previous_id: 1,
                snapshot: Box::new(snapshot)
            }
        ));
        assert!(queue.push(2, Action::Clear));
        assert!(matches!(
            queue.pending.get(&2).map(|work| &work.action),
            Some(Action::ClearRestored { previous_id: 1 })
        ));
        let root = directory()?;
        std::fs::write(root.join("1.json"), b"source").map_err(|error| error.to_string())?;
        let work = queue.pending.remove(&2).ok_or("Missing work")?;
        let Action::ClearRestored { previous_id } = work.action else {
            return Err("Missing source clear".into());
        };
        clear_snapshot(&root, 2).and_then(|_| clear_snapshot(&root, previous_id))?;
        assert!(!root.join("1.json").exists());
        std::fs::remove_dir_all(root).map_err(|e| e.to_string())?;
        Ok(())
    }
    #[test]
    fn in_flight_restoration_cleanup_survives_newer_clear_and_write() -> Result<(), String> {
        for clear in [false, true] {
            let mut queue = Queue::default();
            let mut snapshot = sample()?;
            snapshot.id = 2;
            queue.push(
                2,
                Action::Restore {
                    previous_id: 1,
                    snapshot: Box::new(snapshot.clone()),
                },
            );
            let work = queue.pending.remove(&2).ok_or("Missing work")?;
            queue.push(
                2,
                if clear {
                    Action::Clear
                } else {
                    Action::Write(Box::new(snapshot))
                },
            );
            queue.finish(2, work, Err("old failure".into()));
            assert_eq!(queue.handoffs.get(&2), Some(&1));
            assert!(matches!(
                queue.pending.get(&2).map(|work| &work.action),
                Some(
                    Action::Restore { previous_id: 1, .. }
                        | Action::ClearRestored { previous_id: 1 }
                )
            ));
        }
        Ok(())
    }
    #[test]
    fn recovery_load_ignores_clutter_and_chooses_one_newest_snapshot_per_id() -> Result<(), String>
    {
        let root = directory()?;
        for index in 0..120 {
            std::fs::write(root.join(format!(".{index}.tmp")), b"not recovery")
                .map_err(|e| e.to_string())?;
        }
        let mut snapshot = sample()?;
        std::fs::write(
            root.join("1.json"),
            serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        snapshot.version = 5;
        let bytes = encode(&snapshot)?;
        std::fs::write(root.join("1.trinezip"), &bytes).map_err(|e| e.to_string())?;
        std::fs::write(root.join("1.trinezip.bak"), &bytes).map_err(|e| e.to_string())?;
        std::fs::write(root.join("99.trinezip"), &bytes).map_err(|e| e.to_string())?;
        let loaded = load_at(&root);
        assert_eq!(loaded.len(), 1);
        assert_eq!((loaded[0].id, loaded[0].version), (1, 5));
        std::fs::remove_dir_all(root).map_err(|e| e.to_string())?;
        Ok(())
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
