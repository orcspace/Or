use crate::native::Document;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, SyncSender},
    },
    thread::JoinHandle,
    time::SystemTime,
};
pub struct FileLock {
    path: PathBuf,
    key: String,
    handle: Option<std::fs::File>,
}
impl FileLock {
    fn acquire(path: &Path) -> Result<Self, String> {
        use std::io::Write;
        let lock_path = Self::marker_path(path)?;
        let key = format!(
            "TrineDocs {} {}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|t| t.as_nanos())
                .unwrap_or_default()
        );
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(1);
        }
        let mut f = options
            .open(&lock_path)
            .map_err(|e| format!("Не удалось получить блокировку: {e}"))?;
        if let Err(e) = f.write_all(key.as_bytes()).and_then(|_| f.sync_all()) {
            drop(f);
            let _ = std::fs::remove_file(&lock_path);
            return Err(e.to_string());
        }
        Ok(Self {
            path: lock_path,
            key,
            handle: Some(f),
        })
    }
    pub fn marker_path(path: &Path) -> Result<PathBuf, String> {
        let name = path.file_name().ok_or("Нет имени файла")?.to_string_lossy();
        Ok(path.with_file_name(format!("~{}{name}", "$")))
    }
    fn preview(path: &Path) -> Result<Option<Vec<u8>>, String> {
        let marker = Self::marker_path(path)?;
        match std::fs::metadata(&marker) {
            Ok(_) => crate::util::read_limited(&marker, 65536).map(Some),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    fn take_over(path: &Path, expected: Option<&[u8]>) -> Result<Self, String> {
        let current = Self::preview(path)?;
        if current.as_deref() != expected {
            return Err("Блокировка изменилась. Повторите проверку.".into());
        }
        if current.is_some() {
            std::fs::remove_file(Self::marker_path(path)?).map_err(|e| {
                format!(
                    "Блокировка используется другой программой или нет прав: {e}. Сохраните копию."
                )
            })?;
        }
        Self::acquire(path)
    }
}
impl Drop for FileLock {
    fn drop(&mut self) {
        let owned = std::fs::read_to_string(&self.path).is_ok_and(|s| s == self.key);
        drop(self.handle.take());
        if owned && std::fs::read_to_string(&self.path).is_ok_and(|s| s == self.key) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReloadTarget {
    pub tab_id: u64,
    pub base_version: u64,
    pub path: PathBuf,
}
pub enum Job {
    SaveSettings,
    PreviewRegistration(crate::mcp::register::Client),
    ApplyRegistration(crate::mcp::register::Preview),
    PickOpen,
    PickSave {
        id: u64,
        /// File name without extension proposed in the dialog.
        name: String,
        /// Offered formats `(extension, label)`, the default first.
        formats: Vec<(String, String)>,
        copy: bool,
        export: bool,
    },
    PreviewUnlock {
        id: u64,
        path: PathBuf,
    },
    Unlock {
        id: u64,
        path: PathBuf,
        marker: Option<Vec<u8>>,
        expected: Option<SystemTime>,
    },
    Open(PathBuf),
    Reload(ReloadTarget),
    Save {
        id: u64,
        version: u64,
        path: PathBuf,
        document: Box<Document>,
        backup: bool,
        copy: bool,
        export: bool,
        expected: Option<SystemTime>,
        print: bool,
        owns_lock: bool,
        source_path: Option<PathBuf>,
    },
    Watch(Vec<(u64, PathBuf)>),
    Stop,
}
#[allow(
    clippy::large_enum_variant,
    reason = "Bounded file-worker queue; document buffers are already allocated on the heap."
)]
pub enum Event {
    RegistrationPreview(crate::mcp::register::Preview),
    Registered,
    Canceled {
        id: u64,
    },
    UnlockPreview {
        id: u64,
        path: PathBuf,
        marker: Option<Vec<u8>>,
    },
    Unlocked {
        id: u64,
        path: PathBuf,
        lock: FileLock,
    },
    PickedOpen(PathBuf),
    PickedSave {
        id: u64,
        path: PathBuf,
        copy: bool,
        export: bool,
    },
    Opened {
        reload: Option<ReloadTarget>,
        path: PathBuf,
        document: Document,
        lock: Option<FileLock>,
        stamp: Option<SystemTime>,
    },
    ReloadFailed {
        target: ReloadTarget,
        message: String,
    },
    Saved {
        id: u64,
        version: u64,
        path: PathBuf,
        copy: bool,
        export: bool,
        stamp: Option<SystemTime>,
        lock: Option<FileLock>,
    },
    Watched(Vec<(u64, Option<SystemTime>)>),
    Error {
        message: String,
        id: Option<u64>,
    },
}
pub struct Worker {
    pub tx: SyncSender<Job>,
    pub rx: Receiver<Event>,
    thread: Option<JoinHandle<()>>,
    pending_settings: Arc<Mutex<Option<crate::settings::Settings>>>,
}
pub fn stamp(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).ok().and_then(|m| m.modified().ok())
}
fn export_unchanged(
    path: &Path,
    expected: Option<SystemTime>,
    export: impl FnOnce() -> Result<Vec<u8>, String>,
) -> Result<Vec<u8>, String> {
    fn checked_stamp(path: &Path) -> Result<Option<SystemTime>, String> {
        match std::fs::metadata(path) {
            Ok(metadata) => metadata
                .modified()
                .map(Some)
                .map_err(|error| error.to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }
    let before = checked_stamp(path)?;
    if expected.is_some() && before != expected {
        return Err("Файл изменён на диске: сохраните копию или перезагрузите".into());
    }
    let bytes = export()?;
    if checked_stamp(path)? != before {
        return Err("Файл изменился во время экспорта: сохраните копию или перезагрузите".into());
    }
    Ok(bytes)
}
fn ensure_copy_target(source: Option<&Path>, target: &Path) -> Result<(), String> {
    if let Some(source) = source {
        let same = source == target
            || std::fs::canonicalize(source)
                .ok()
                .zip(std::fs::canonicalize(target).ok())
                .is_some_and(|(source, target)| source == target);
        if same {
            return Err("Для копии выберите другой файл; исходный файл сохранён.".into());
        }
    }
    Ok(())
}
fn open_event(path: PathBuf, reload: Option<ReloadTarget>) -> Event {
    let path = std::fs::canonicalize(&path).unwrap_or(path);
    let lock = if reload.is_some() {
        None
    } else {
        FileLock::acquire(&path).ok()
    };
    let before = stamp(&path);
    let result = crate::native::open(&path).and_then(|document| {
        let after = stamp(&path);
        if before != after {
            return Err("Файл изменился во время чтения; повторите открытие".into());
        }
        Ok((document, after))
    });
    match result {
        Ok((document, stamp)) => Event::Opened {
            reload,
            path,
            document,
            lock,
            stamp,
        },
        Err(message) => match reload {
            Some(target) => Event::ReloadFailed { target, message },
            None => Event::Error { message, id: None },
        },
    }
}
impl Worker {
    pub fn start(ctx: egui::Context) -> Self {
        Self::start_inner(ctx, true)
    }
    #[cfg(test)]
    pub(crate) fn start_for_test(ctx: egui::Context) -> Self {
        Self::start_inner(ctx, false)
    }
    fn start_inner(ctx: egui::Context, persist_settings: bool) -> Self {
        let (tx, jobs) = mpsc::sync_channel(16);
        let (events, rx) = mpsc::channel();
        let pending_settings = Arc::new(Mutex::new(None::<crate::settings::Settings>));
        let worker_settings = pending_settings.clone();
        let thread = std::thread::spawn(move || {
            while let Ok(job) = jobs.recv() {
                let settings = worker_settings
                    .lock()
                    .ok()
                    .and_then(|mut pending| pending.take());
                if let Some(settings) = settings
                    && persist_settings
                    && let Err(message) = settings.save()
                {
                    if events
                        .send(Event::Error {
                            message: format!("Не удалось сохранить настройки: {message}"),
                            id: None,
                        })
                        .is_err()
                    {
                        break;
                    }
                    ctx.request_repaint();
                }
                let result = match job {
                    Job::Stop => break,
                    Job::SaveSettings => None,
                    Job::PreviewRegistration(client) => {
                        let result =
                            std::env::current_exe()
                                .map_err(|e| e.to_string())
                                .and_then(|p| {
                                    crate::mcp::register::preview(
                                        client,
                                        &p.with_file_name("trinedocs-mcp.exe"),
                                    )
                                });
                        Some(
                            result
                                .map(Event::RegistrationPreview)
                                .unwrap_or_else(|message| Event::Error { message, id: None }),
                        )
                    }
                    Job::ApplyRegistration(p) => Some(
                        crate::mcp::register::apply(&p)
                            .map(|_| Event::Registered)
                            .unwrap_or_else(|message| Event::Error { message, id: None }),
                    ),
                    Job::PickOpen => rfd::FileDialog::new()
                        .add_filter(
                            "TrineDocs / Office",
                            &[
                                "docx",
                                "xlsx",
                                "pptx",
                                "trinedoc",
                                "trinesheet",
                                "trineslide",
                                "dotx",
                                "docm",
                                "odt",
                                "ott",
                                "fodt",
                                "xlsm",
                                "xltx",
                                "ppsx",
                                "potx",
                                "pptm",
                                "html",
                                "htm",
                                "txt",
                                "lock",
                                "md",
                                "csv",
                                "tsv",
                                "ods",
                                "xls",
                                "xlsb",
                            ],
                        )
                        .pick_file()
                        .map(Event::PickedOpen)
                        .or(Some(Event::Canceled { id: 0 })),
                    Job::PickSave {
                        id,
                        name,
                        formats,
                        copy,
                        export,
                    } => Some({
                        let default = formats.first().map(|f| f.0.clone()).unwrap_or_default();
                        let mut dialog =
                            rfd::FileDialog::new().set_file_name(format!("{name}.{default}"));
                        // The file type list of the dialog decides the format, as in Office.
                        for (extension, label) in &formats {
                            dialog = dialog.add_filter(
                                format!("{label} (*.{extension})"),
                                &[extension.as_str()],
                            );
                        }
                        dialog
                            .save_file()
                            .map(|path| Event::PickedSave {
                                id,
                                path: std::fs::canonicalize(&path).unwrap_or(path),
                                copy,
                                export,
                            })
                            .unwrap_or(Event::Canceled { id })
                    }),
                    Job::PreviewUnlock { id, path } => Some(
                        FileLock::preview(&path)
                            .map(|marker| Event::UnlockPreview { id, path, marker })
                            .unwrap_or_else(|message| Event::Error {
                                message,
                                id: Some(id),
                            }),
                    ),
                    Job::Unlock {
                        id,
                        path,
                        marker,
                        expected,
                    } => {
                        let result = if stamp(&path) != expected {
                            Err(
                                "Файл изменился на диске. Сохраните копию или перезагрузите его."
                                    .into(),
                            )
                        } else {
                            FileLock::take_over(&path, marker.as_deref())
                                .map(|lock| Event::Unlocked { id, path, lock })
                        };
                        Some(result.unwrap_or_else(|message| Event::Error {
                            message,
                            id: Some(id),
                        }))
                    }
                    Job::Open(path) => Some(open_event(path, None)),
                    Job::Reload(target) => Some(open_event(target.path.clone(), Some(target))),
                    Job::Save {
                        id,
                        version,
                        path,
                        document,
                        backup,
                        copy,
                        export,
                        expected,
                        print,
                        owns_lock,
                        source_path,
                    } => {
                        let result = (|| -> Result<Event, String> {
                            if copy {
                                ensure_copy_target(source_path.as_deref(), &path)?;
                            }
                            let lock = if owns_lock {
                                None
                            } else {
                                if let Some(parent) = path.parent() {
                                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                                }
                                Some(FileLock::acquire(&path)?)
                            };
                            let ext = crate::native::extension(&path);
                            // A PNG export of a deck: the chosen file gets the first shown slide,
                            // the other shown slides go next to it as `Имя (слайд N).png`.
                            let mut more_slides = Vec::new();
                            let bytes =
                                export_unchanged(&path, expected, || match document.as_ref() {
                                    crate::native::Document::Slides(deck) if ext == "png" => {
                                        let mut pictures =
                                            crate::native::slide_pictures(deck)?.into_iter();
                                        let (_, first) = pictures
                                            .next()
                                            .ok_or("В презентации нет показываемых слайдов")?;
                                        more_slides = pictures.collect();
                                        Ok(first)
                                    }
                                    _ => crate::native::export(&document, &ext),
                                })?;
                            crate::util::atomic_write(&path, &bytes, backup)?;
                            for (number, picture) in &more_slides {
                                crate::native::write_slide_picture(&path, *number, picture)?;
                            }
                            let lock = if !copy && !export { lock } else { None };
                            if print {
                                #[cfg(windows)]
                                {
                                    use std::os::windows::process::CommandExt;
                                    let _ = std::process::Command::new("explorer.exe")
                                        .arg(&path)
                                        .creation_flags(0x08000000)
                                        .spawn();
                                }
                            }
                            let path = std::fs::canonicalize(&path).map_err(|e| e.to_string())?;
                            Ok(Event::Saved {
                                id,
                                version,
                                stamp: stamp(&path),
                                path,
                                copy,
                                export,
                                lock,
                            })
                        })();
                        Some(result.unwrap_or_else(|message| Event::Error {
                            message,
                            id: Some(id),
                        }))
                    }
                    Job::Watch(paths) => Some(Event::Watched(
                        paths.into_iter().map(|(id, p)| (id, stamp(&p))).collect(),
                    )),
                };
                if let Some(e) = result {
                    if events.send(e).is_err() {
                        break;
                    }
                    ctx.request_repaint();
                }
            }
        });
        Self {
            tx,
            rx,
            thread: Some(thread),
            pending_settings,
        }
    }
    pub fn save_settings(&self, settings: crate::settings::Settings) -> Result<(), String> {
        *self
            .pending_settings
            .lock()
            .map_err(|_| "Очередь настроек недоступна")? = Some(settings);
        match self.tx.try_send(Job::SaveSettings) {
            Ok(()) | Err(mpsc::TrySendError::Full(_)) => Ok(()),
            Err(mpsc::TrySendError::Disconnected(_)) => Err("Файловый поток остановлен".into()),
        }
    }
    pub fn send(&self, job: Job) -> Result<(), String> {
        self.tx
            .try_send(job)
            .map_err(|_| "Очередь файлов занята; повторите".into())
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.tx.send(Job::Stop);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[cfg(test)]
mod lock_tests {
    use super::*;
    fn directory() -> Result<PathBuf, String> {
        let name = format!(
            "trinedocs-lock-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos()
        );
        let dir = std::env::temp_dir().join(name);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Ok(dir)
    }
    #[test]
    fn copy_cannot_replace_a_canonical_alias_of_its_source() -> Result<(), String> {
        let root = directory()?;
        std::fs::create_dir_all(root.join("nested")).map_err(|error| error.to_string())?;
        let source = root.join("original.docx");
        std::fs::write(&source, b"source").map_err(|error| error.to_string())?;
        let alias = root.join("nested").join("..").join("original.docx");
        assert!(ensure_copy_target(Some(&source), &alias).is_err());
        assert!(ensure_copy_target(Some(&source), &root.join("copy.docx")).is_ok());
        assert_eq!(
            std::fs::read(&source).map_err(|error| error.to_string())?,
            b"source"
        );
        std::fs::remove_dir_all(root).map_err(|error| error.to_string())?;
        Ok(())
    }

    #[test]
    fn save_as_detects_target_creation_or_modification_during_export() -> Result<(), String> {
        let dir = directory()?;
        let path = dir.join("target.docx");
        assert!(
            export_unchanged(&path, None, || {
                std::fs::write(&path, b"other application").map_err(|e| e.to_string())?;
                Ok(b"our export".to_vec())
            })
            .is_err()
        );
        assert_eq!(
            std::fs::read(&path).map_err(|e| e.to_string())?,
            b"other application"
        );
        let before = stamp(&path).ok_or("Missing timestamp")?;
        assert!(
            export_unchanged(&path, None, || {
                let file = std::fs::OpenOptions::new()
                    .write(true)
                    .open(&path)
                    .map_err(|e| e.to_string())?;
                file.set_modified(before + std::time::Duration::from_secs(5))
                    .map_err(|e| e.to_string())?;
                Ok(b"our export".to_vec())
            })
            .is_err()
        );
        assert_eq!(
            export_unchanged(&path, stamp(&path), || Ok(b"unchanged".to_vec()))?,
            b"unchanged"
        );
        std::fs::remove_dir_all(dir).map_err(|e| e.to_string())?;
        Ok(())
    }
    #[test]
    fn explicit_stale_lock_takeover_is_guarded_and_released() -> Result<(), String> {
        let dir = directory()?;
        let path = dir.join("book.docx");
        let marker = FileLock::marker_path(&path)?;
        std::fs::write(&marker, b"stale marker").map_err(|e| e.to_string())?;
        assert!(FileLock::acquire(&path).is_err());
        assert!(FileLock::take_over(&path, Some(b"different")).is_err());
        assert_eq!(
            std::fs::read(&marker).map_err(|e| e.to_string())?,
            b"stale marker"
        );
        let lock = FileLock::take_over(&path, Some(b"stale marker"))?;
        assert!(FileLock::acquire(&path).is_err());
        drop(lock);
        assert!(!marker.exists());
        std::fs::remove_dir(&dir).map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(windows)]
    #[test]
    fn a_live_windows_lock_cannot_be_taken_over() -> Result<(), String> {
        let dir = directory()?;
        let path = dir.join("live.docx");
        let lock = FileLock::acquire(&path)?;
        let marker = FileLock::preview(&path)?.ok_or("Нет блокировки")?;
        assert!(FileLock::take_over(&path, Some(&marker)).is_err());
        assert!(FileLock::preview(&path)?.is_some());
        drop(lock);
        assert!(FileLock::preview(&path)?.is_none());
        std::fs::remove_dir(&dir).map_err(|e| e.to_string())?;
        Ok(())
    }
}
