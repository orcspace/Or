use crate::native::Document;
use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender},
    thread::JoinHandle,
    time::SystemTime,
};
pub struct FileLock {
    path: PathBuf,
    key: String,
}
impl FileLock {
    fn acquire(path: &Path) -> Result<Self, String> {
        use std::io::Write;
        let name = path.file_name().ok_or("Нет имени файла")?.to_string_lossy();
        let lock_path = path.with_file_name(format!("~{}{name}", "$"));
        let key = format!(
            "TrineDocs {} {}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|t| t.as_nanos())
                .unwrap_or_default()
        );
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .map_err(|e| e.to_string())?;
        if let Err(e) = f.write_all(key.as_bytes()).and_then(|_| f.sync_all()) {
            drop(f);
            let _ = std::fs::remove_file(&lock_path);
            return Err(e.to_string());
        }
        Ok(Self {
            path: lock_path,
            key,
        })
    }
}
impl Drop for FileLock {
    fn drop(&mut self) {
        if std::fs::read_to_string(&self.path).is_ok_and(|s| s == self.key) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
pub enum Job {
    PreviewRegistration(crate::mcp::register::Client),
    ApplyRegistration(crate::mcp::register::Preview),
    PickOpen,
    PickSave {
        id: u64,
        extension: String,
        copy: bool,
        export: bool,
    },
    Open(PathBuf),
    Save {
        id: u64,
        version: u64,
        path: PathBuf,
        document: Document,
        backup: bool,
        copy: bool,
        export: bool,
        expected: Option<SystemTime>,
        print: bool,
        owns_lock: bool,
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
    PickedOpen(PathBuf),
    PickedSave {
        id: u64,
        path: PathBuf,
        copy: bool,
        export: bool,
    },
    Opened {
        path: PathBuf,
        document: Document,
        lock: Option<FileLock>,
        stamp: Option<SystemTime>,
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
}
pub fn stamp(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).ok().and_then(|m| m.modified().ok())
}
impl Worker {
    pub fn start(ctx: egui::Context) -> Self {
        let (tx, jobs) = mpsc::sync_channel(16);
        let (events, rx) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            while let Ok(job) = jobs.recv() {
                let result = match job {
                    Job::Stop => break,
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
                                "trinedoc",
                                "trinesheet",
                                "trineslide",
                                "docx",
                                "xlsx",
                                "pptx",
                                "txt",
                                "md",
                                "csv",
                                "tsv",
                                "ods",
                                "xls",
                                "xlsb",
                            ],
                        )
                        .pick_file()
                        .map(Event::PickedOpen),
                    Job::PickSave {
                        id,
                        extension,
                        copy,
                        export,
                    } => Some(
                        rfd::FileDialog::new()
                            .set_file_name(format!("Без названия.{extension}"))
                            .add_filter(&extension, &[&extension])
                            .save_file()
                            .map(|path| Event::PickedSave {
                                id,
                                path: std::fs::canonicalize(&path).unwrap_or(path),
                                copy,
                                export,
                            })
                            .unwrap_or(Event::Canceled { id }),
                    ),
                    Job::Open(path) => {
                        let path = std::fs::canonicalize(&path).unwrap_or(path);
                        let lock = FileLock::acquire(&path).ok();
                        Some(match crate::native::open(&path) {
                            Ok(document) => Event::Opened {
                                stamp: stamp(&path),
                                path,
                                document,
                                lock,
                            },
                            Err(message) => Event::Error { message, id: None },
                        })
                    }
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
                    } => {
                        let result = (|| -> Result<Event, String> {
                            let lock = if owns_lock {
                                None
                            } else {
                                if let Some(parent) = path.parent() {
                                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                                }
                                Some(FileLock::acquire(&path)?)
                            };
                            if expected.is_some() && stamp(&path) != expected {
                                return Err(
                                    "Файл изменён на диске: сохраните копию или перезагрузите"
                                        .into(),
                                );
                            }
                            let bytes =
                                crate::native::export(&document, &crate::native::extension(&path))?;
                            crate::util::atomic_write(&path, &bytes, backup)?;
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
