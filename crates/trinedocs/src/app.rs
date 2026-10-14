use crate::{
    file_io::{Event, FileLock, Job, ReloadTarget, Worker},
    native::Document,
    ops::{Author, Op, Tab},
    settings::Settings,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    time::{Instant, SystemTime},
};
#[derive(Clone)]
enum Pending {
    Register(crate::mcp::register::Preview),
    Close(u64),
    Exit,
    Unlock {
        id: u64,
        path: PathBuf,
        marker: Option<Vec<u8>>,
    },
    SaveFormat {
        id: u64,
        path: PathBuf,
        copy: bool,
        export: bool,
    },
}
fn palette_commands(has_document: bool, query: &str) -> Vec<(&'static str, &'static str)> {
    let query = query.trim().to_lowercase();
    [
        ("docs", "Создать документ"),
        ("sheets", "Создать таблицу"),
        ("slides", "Создать презентацию"),
        ("open", "Открыть файл · Ctrl+O"),
        ("settings", "Настройки"),
        ("save", "Сохранить · Ctrl+S"),
        ("saveas", "Сохранить как…"),
        ("copy", "Сохранить копию…"),
        ("pdf", "Экспорт в PDF"),
        ("close", "Закрыть документ · Ctrl+W"),
    ]
    .into_iter()
    .filter(|(key, label)| {
        (has_document || matches!(*key, "docs" | "sheets" | "slides" | "open" | "settings"))
            && label.to_lowercase().contains(&query)
    })
    .collect()
}
fn fresh_tab_id(tabs: &[Tab], next_id: u64) -> Result<(u64, u64), String> {
    let following = next_id.checked_add(1).ok_or("Исчерпаны ID документов")?;
    if next_id == 0 || tabs.iter().any(|t| t.id == next_id) {
        return Err("Конфликт ID документов".into());
    }
    Ok((next_id, following))
}
fn prepare_reload(
    tabs: &[Tab],
    next_id: u64,
    target: &ReloadTarget,
    path: &std::path::Path,
    document: Document,
) -> Result<(usize, Tab, u64), String> {
    let index = tabs
        .iter()
        .position(|t| t.id == target.tab_id)
        .ok_or("conflict: документ уже закрыт")?;
    let old = &tabs[index];
    if old.version != target.base_version
        || old.path.as_deref() != Some(target.path.as_path())
        || path != target.path.as_path()
    {
        return Err("conflict: во время загрузки документ изменился; новые правки сохранены. Повторите перезагрузку.".into());
    }
    let (id, following) = fresh_tab_id(tabs, next_id)?;
    document.validate()?;
    let tab = Tab::new(id, document, old.title.clone(), Some(path.to_path_buf()));
    Ok((index, tab, following))
}
enum PreparedOpen {
    Activate(u64),
    Install {
        tab: Box<Tab>,
        following: u64,
        replace: Option<(usize, u64)>,
    },
}
fn prepare_open(
    tabs: &[Tab],
    next_id: u64,
    reload: Option<&ReloadTarget>,
    path: &std::path::Path,
    document: Document,
) -> Result<PreparedOpen, String> {
    if let Some(target) = reload {
        let (index, tab, following) = prepare_reload(tabs, next_id, target, path, document)?;
        return Ok(PreparedOpen::Install {
            tab: Box::new(tab),
            following,
            replace: Some((index, target.tab_id)),
        });
    }
    if let Some(t) = tabs.iter().find(|t| t.path.as_deref() == Some(path)) {
        return Ok(PreparedOpen::Activate(t.id));
    }
    let (id, following) = fresh_tab_id(tabs, next_id)?;
    document.validate()?;
    let title = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Без названия".into());
    Ok(PreparedOpen::Install {
        tab: Box::new(Tab::new(id, document, title, Some(path.to_path_buf()))),
        following,
        replace: None,
    })
}
fn commit_open(tabs: &mut Vec<Tab>, next_id: &mut u64, prepared: PreparedOpen) -> u64 {
    match prepared {
        PreparedOpen::Activate(id) => id,
        PreparedOpen::Install {
            tab,
            following,
            replace,
        } => {
            let id = tab.id;
            *next_id = following;
            if let Some((index, _)) = replace {
                tabs[index] = *tab;
            } else {
                tabs.push(*tab);
            }
            id
        }
    }
}
pub struct App {
    #[cfg(test)]
    shell_rects: BTreeMap<(u64, &'static str), egui::Rect>,
    editor_ctx: egui::Context,
    deferred: Vec<(u64, &'static str)>,
    smoke: Option<crate::ui_smoke::State>,
    tabs: Vec<Tab>,
    active: Option<u64>,
    next_id: u64,
    settings: Settings,
    worker: Worker,
    recovery: crate::recovery::Recovery,
    locks: BTreeMap<u64, FileLock>,
    stamps: BTreeMap<u64, Option<SystemTime>>,
    recoveries: Vec<crate::recovery::Snapshot>,
    mcp: Option<crate::mcp::Handle>,
    bridge: Option<std::sync::mpsc::Receiver<crate::mcp::BridgeRequest>>,
    allowed: BTreeSet<String>,
    clients: BTreeSet<String>,
    mcp_saves: BTreeMap<u64, tokio::sync::oneshot::Sender<Result<Value, String>>>,
    pending_mcp_closes: Vec<crate::mcp::BridgeRequest>,
    pending: Option<Pending>,
    close_after_save: Option<u64>,
    status: String,
    error: String,
    show_settings: bool,
    show_ai: bool,
    /// 0 = chat, 1 = AI change review.
    ai_tab: usize,
    chat: crate::chat::Chat,
    palette: bool,
    query: String,
    palette_query: String,
    palette_focus: bool,
    palette_tail: Option<Vec<egui::Event>>,
    last_snapshot: Instant,
    last_watch: Instant,
    snapshot_versions: BTreeMap<u64, u64>,
    disk_changed: Option<u64>,
    last_agent: Option<(u64, String, Instant)>,
    allow_exit: bool,
    exit_after_save: bool,
    saving: BTreeSet<u64>,
    picking: BTreeSet<u64>,
    opening: bool,
    reloading: BTreeMap<u64, ReloadTarget>,
}
impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let smoke = crate::ui_smoke::State::from_args();
        let settings = if smoke.is_some() {
            Settings {
                mcp: false,
                theme: if std::env::args().any(|a| a == "--ui-smoke-dark") {
                    "dark"
                } else {
                    "light"
                }
                .into(),
                ..Default::default()
            }
        } else {
            Settings::load()
        };
        let chat = crate::chat::Chat::with_prefs(&settings.chat_models, &settings.chat_efforts);
        let ctx = &cc.egui_ctx;
        if std::env::args().any(|a| a == "--background") {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
        let font_error = crate::fonts::install(ctx).err().unwrap_or_default();
        ctx.set_zoom_factor(settings.scale.clamp(0.75, 2.0));
        let (mcp, bridge) = if settings.mcp {
            let (m, b) = crate::mcp::Handle::start(ctx.clone());
            (Some(m), Some(b))
        } else {
            (None, None)
        };
        let next_id = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(1);
        let app = Self {
            #[cfg(test)]
            shell_rects: BTreeMap::new(),
            editor_ctx: ctx.clone(),
            deferred: Vec::new(),
            smoke,
            tabs: Vec::new(),
            active: None,
            next_id,
            settings,
            worker: Worker::start(ctx.clone()),
            recovery: crate::recovery::Recovery::start(),
            locks: BTreeMap::new(),
            stamps: BTreeMap::new(),
            recoveries: if std::env::args().any(|a| a == "--ui-smoke-test") {
                Vec::new()
            } else {
                crate::recovery::Recovery::load()
            },
            mcp,
            bridge,
            allowed: BTreeSet::new(),
            clients: BTreeSet::new(),
            mcp_saves: BTreeMap::new(),
            pending_mcp_closes: Vec::new(),
            pending: None,
            close_after_save: None,
            status: "Готово".into(),
            error: font_error,
            show_settings: false,
            show_ai: std::env::args().any(|a| a == "--ui-smoke-compact"),
            ai_tab: 0,
            chat,
            palette: false,
            query: String::new(),
            palette_query: String::new(),
            palette_focus: false,
            palette_tail: None,
            last_snapshot: Instant::now(),
            last_watch: Instant::now(),
            snapshot_versions: BTreeMap::new(),
            disk_changed: None,
            last_agent: None,
            allow_exit: false,
            exit_after_save: false,
            saving: BTreeSet::new(),
            picking: BTreeSet::new(),
            opening: false,
            reloading: BTreeMap::new(),
        };
        for arg in std::env::args().skip(1).filter(|s| !s.starts_with('-')) {
            if app.smoke.is_none() {
                let _ = app.worker.send(Job::Open(PathBuf::from(arg)));
            }
        }
        app
    }
    fn queue_palette(&mut self, ctx: &egui::Context, boundary: Option<usize>) {
        let tail = ctx.input_mut(|i| {
            let split = boundary
                .map_or(i.events.len(), |index| index + 1)
                .min(i.events.len());
            let tail = i.events.split_off(split);
            if let Some(index) = boundary
                && index < i.events.len()
            {
                i.events.remove(index);
            }
            tail
        });
        self.palette_tail = Some(tail);
    }
    fn queue_palette_click(&mut self, ctx: &egui::Context) {
        let boundary = ctx.input(|i| {
            i.events.iter().position(|e| {
                matches!(
                    e,
                    egui::Event::PointerButton {
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        ..
                    }
                )
            })
        });
        self.queue_palette(ctx, boundary);
    }
    fn open_palette(&mut self, ctx: &egui::Context) {
        let id = egui::Id::new("command-palette-query");
        if egui::text_edit::TextEditState::load(ctx, id).is_none() {
            egui::text_edit::TextEditState::default().store(ctx, id);
        }
        ctx.memory_mut(|m| m.request_focus(id));
        self.palette = true;
        self.palette_focus = true;
    }
    fn tr<'a>(&self, s: &'a str) -> &'a str {
        crate::i18n::tr(s, &self.settings.language)
    }
    fn create(&mut self, kind: &str) {
        if let Some(id) = self.active
            && let Err(e) = self.flush_pending(id)
        {
            self.error = e;
            return;
        }
        match Document::create(kind) {
            Ok(d) => {
                let (id, following) = match fresh_tab_id(&self.tabs, self.next_id) {
                    Ok(ids) => ids,
                    Err(e) => {
                        self.error = e;
                        return;
                    }
                };
                self.next_id = following;
                let mut t = Tab::new(
                    id,
                    d,
                    self.tr(match kind {
                        "docs" => "Document",
                        "sheets" => "Spreadsheet",
                        _ => "Presentation",
                    })
                    .into(),
                    None,
                );
                t.saved_version = u64::MAX;
                self.tabs.push(t);
                self.active = Some(id);
            }
            Err(e) => self.error = e,
        }
    }
    fn send(&mut self, job: Job) {
        let picker = matches!(&job, Job::PickOpen);
        if picker && self.opening {
            self.error = "Диалог открытия уже открыт".into();
            return;
        }
        match self.worker.send(job) {
            Ok(()) => {
                if picker {
                    self.opening = true;
                }
            }
            Err(e) => self.error = e,
        }
    }
    fn current(&self) -> Option<&Tab> {
        self.tabs.iter().find(|t| Some(t.id) == self.active)
    }
    fn flush_pending(&mut self, id: u64) -> Result<(), String> {
        let Some(t) = self.tabs.iter_mut().find(|t| t.id == id) else {
            return Ok(());
        };
        if let Document::Sheets(book) = &t.document {
            let mut book = book.clone();
            if crate::sheets::editor::commit_pending(&self.editor_ctx, id, &mut book)? {
                t.apply(Op::Replace {
                    document: Document::Sheets(book),
                    label: "Редактирование".into(),
                    author: Author::User,
                    base_version: None,
                })?;
            }
        }
        Ok(())
    }
    fn save(&mut self, id: u64, copy: bool, export: bool, choose: bool, extension: Option<&str>) {
        if self.picking.contains(&id) {
            self.error = "Дождитесь выбора файла".into();
            return;
        }
        if let Err(e) = self.flush_pending(id) {
            self.fail_save(id, e);
            return;
        }
        if self.reloading.contains_key(&id) {
            self.fail_save(id, "busy: дождитесь завершения перезагрузки".into());
            return;
        }
        if self.saving.contains(&id) {
            self.error = "Сохранение уже выполняется".into();
            return;
        }
        let Some(t) = self.tabs.iter().find(|t| t.id == id) else {
            return;
        };
        // Files in the legacy TrineDocs format are re-saved through "Save as" in Office format.
        let legacy = t
            .path
            .as_ref()
            .is_some_and(|p| crate::native::extension(p) == t.document.native_extension());
        if choose || t.path.is_none() || copy || export || legacy {
            let ext = extension.unwrap_or(t.document.extension());
            let mut formats: Vec<(String, String)> = t
                .document
                .save_formats()
                .iter()
                .map(|(e, l)| (e.to_string(), l.to_string()))
                .collect();
            // The requested format goes first so the dialog preselects it.
            if let Some(i) = formats.iter().position(|(e, _)| e == ext) {
                let chosen = formats.remove(i);
                formats.insert(0, chosen);
            } else {
                formats.insert(0, (ext.to_string(), ext.to_uppercase()));
            }
            let name = t
                .path
                .as_ref()
                .and_then(|p| p.file_stem())
                .and_then(|s| s.to_str())
                .map(str::to_string)
                .unwrap_or_else(|| "Без названия".into());
            match self.worker.send(Job::PickSave {
                id,
                name,
                formats,
                copy,
                export,
            }) {
                Ok(()) => {
                    self.picking.insert(id);
                }
                Err(e) => self.fail_save(id, e),
            }
        } else if let Some(path) = t.path.clone() {
            if crate::native::extension(&path) != t.document.extension() {
                self.pending = Some(Pending::SaveFormat {
                    id,
                    path,
                    copy,
                    export,
                });
            } else {
                self.write(id, path, copy, export, false);
            }
        }
    }
    fn fail_save(&mut self, id: u64, message: String) {
        self.error = message.clone();
        self.close_after_save = None;
        self.exit_after_save = false;
        if let Some(reply) = self.mcp_saves.remove(&id) {
            let _ = reply.send(Err(message));
        }
    }
    fn reload(&mut self, id: u64) {
        if let Err(e) = self.flush_pending(id) {
            self.error = e;
            return;
        }
        if self.picking.contains(&id) {
            self.error = "Дождитесь выбора файла".into();
            return;
        }
        if self.saving.contains(&id) || self.reloading.contains_key(&id) || self.pending.is_some() {
            self.error =
                "Завершите сохранение, загрузку или подтверждение перед перезагрузкой".into();
            return;
        }
        let Some(t) = self.tabs.iter().find(|t| t.id == id) else {
            self.disk_changed = None;
            return;
        };
        let Some(path) = t.path.clone() else {
            return;
        };
        let target = ReloadTarget {
            tab_id: id,
            base_version: t.version,
            path,
        };
        match self.worker.send(Job::Reload(target.clone())) {
            Ok(()) => {
                self.reloading.insert(id, target);
                self.disk_changed = None;
                self.status = "Перезагрузка…".into();
            }
            Err(e) => self.error = e,
        }
    }
    fn request_unlock(&mut self, id: u64) {
        if self.pending.is_some() {
            self.error = "Сначала завершите подтверждение".into();
            return;
        }
        if self.picking.contains(&id)
            || self.saving.contains(&id)
            || self.reloading.contains_key(&id)
        {
            self.error = "Дождитесь файловой операции".into();
            return;
        }
        if let Some(path) = self
            .tabs
            .iter()
            .find(|t| t.id == id)
            .and_then(|t| t.path.clone())
        {
            self.send(Job::PreviewUnlock { id, path });
        }
    }
    fn write(&mut self, id: u64, path: PathBuf, copy: bool, export: bool, print: bool) {
        if let Err(e) = self.flush_pending(id) {
            self.fail_save(id, e);
            return;
        }
        if self.reloading.contains_key(&id) || self.saving.contains(&id) {
            self.fail_save(id, "busy: файловая операция уже выполняется".into());
            return;
        }
        let Some(t) = self.tabs.iter().find(|t| t.id == id) else {
            return;
        };
        let expected = if t.path.as_ref() == Some(&path) {
            self.stamps.get(&id).copied().flatten()
        } else {
            None
        };
        if t.save_locked && t.path.as_ref() == Some(&path) {
            self.fail_save(
                id,
                "Файл занят: правки доступны, сохраните копию или снимите оставшуюся блокировку."
                    .into(),
            );
            return;
        }
        let owns_lock = t.path.as_ref() == Some(&path) && self.locks.contains_key(&id);
        let job = Job::Save {
            id,
            version: t.version,
            path,
            document: t.document.clone(),
            backup: self.settings.backup,
            copy,
            export,
            expected,
            print,
            owns_lock,
        };
        match self.worker.send(job) {
            Ok(()) => {
                if !copy
                    && !export
                    && let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id)
                {
                    tab.last_edit = Instant::now()
                        .checked_sub(std::time::Duration::from_secs(2))
                        .unwrap_or_else(Instant::now);
                }
                self.saving.insert(id);
                self.status = "Сохранение…".into();
            }
            Err(e) => {
                self.error = e;
                self.close_after_save = None;
                self.exit_after_save = false;
                if let Some(reply) = self.mcp_saves.remove(&id) {
                    let _ = reply.send(Err(self.error.clone()));
                }
            }
        }
    }
    fn retire_state(&mut self, id: u64) {
        if let Some(reply) = self.mcp_saves.remove(&id) {
            let _ = reply.send(Err("Документ закрыт".into()));
        }
        self.locks.remove(&id);
        self.stamps.remove(&id);
        self.snapshot_versions.remove(&id);
        self.saving.remove(&id);
        self.reloading.remove(&id);
        self.recoveries.retain(|s| s.id != id);
        if matches!(&self.pending, Some(Pending::Close(old) | Pending::SaveFormat { id: old, .. } | Pending::Unlock { id: old, .. }) if *old == id)
        {
            self.pending = None;
        }
        if self.close_after_save == Some(id) {
            self.close_after_save = None;
            self.exit_after_save = false;
        }
        if self.disk_changed == Some(id) {
            self.disk_changed = None;
        }
        if self
            .last_agent
            .as_ref()
            .is_some_and(|(old, _, _)| *old == id)
        {
            self.last_agent = None;
        }
        self.recovery.clear(id);
    }
    fn remove(&mut self, id: u64) {
        if self.saving.contains(&id) || self.picking.contains(&id) {
            self.error = "busy: дождитесь завершения сохранения перед закрытием".into();
            return;
        }
        self.retire_state(id);
        self.tabs.retain(|t| t.id != id);
        if self.active == Some(id) {
            self.active = self.tabs.last().map(|t| t.id);
        }
    }
    fn close(&mut self, id: u64) {
        if let Err(e) = self.flush_pending(id) {
            self.error = e;
            return;
        }
        if self.saving.contains(&id) || self.picking.contains(&id) {
            self.error = "busy: дождитесь завершения сохранения перед закрытием".into();
            return;
        }
        if self.tabs.iter().any(|t| t.id == id && t.dirty()) {
            self.pending = Some(Pending::Close(id));
        } else {
            self.remove(id);
        }
    }
    fn events(&mut self) {
        while let Ok(event) = self.worker.rx.try_recv() {
            match event {
                Event::UnlockPreview { id, path, marker } => {
                    if self.pending.is_none()
                        && self
                            .tabs
                            .iter()
                            .any(|t| t.id == id && t.path.as_ref() == Some(&path) && t.save_locked)
                    {
                        self.pending = Some(Pending::Unlock { id, path, marker });
                    }
                }
                Event::Unlocked { id, path, lock } => {
                    if let Some(t) = self
                        .tabs
                        .iter_mut()
                        .find(|t| t.id == id && t.path.as_ref() == Some(&path))
                    {
                        t.save_locked = false;
                        self.locks.insert(id, lock);
                        self.status = "Блокировка снята · можно сохранить файл".into();
                    }
                }
                Event::RegistrationPreview(p) => {
                    if self.pending.is_none() {
                        self.pending = Some(Pending::Register(p));
                    } else {
                        self.error =
                            "Сначала завершите текущее подтверждение, затем повторите регистрацию."
                                .into();
                    }
                }
                Event::Registered => {
                    self.status = "Клиент MCP зарегистрирован. Перезапустите клиент.".into()
                }
                Event::PickedOpen(path) => {
                    self.opening = false;
                    self.send(Job::Open(path));
                }
                Event::PickedSave {
                    id,
                    path,
                    copy,
                    export,
                } => {
                    self.picking.remove(&id);
                    // A non-Office format (PDF, TXT, PNG…) is an export: the tab keeps its file.
                    let export = export
                        || self.tabs.iter().find(|t| t.id == id).is_some_and(|t| {
                            crate::native::extension(&path) != t.document.office_extension()
                        });
                    if self.pending.is_some() {
                        self.fail_save(
                            id,
                            "Другое подтверждение уже открыто; повторите сохранение.".into(),
                        );
                        continue;
                    }
                    if self.tabs.iter().find(|t| t.id == id).is_some_and(|t| {
                        crate::native::extension(&path) != t.document.extension()
                            && (crate::native::extension(&path) != "pdf"
                                || matches!(&t.document, Document::Slides(deck) if deck.slides.iter().any(|s| s.elements.iter().any(|e| e.rotation != 0.0))))
                    }) {
                        self.pending = Some(Pending::SaveFormat {
                            id,
                            path,
                            copy,
                            export,
                        });
                    } else {
                        self.write(id, path, copy, export, false);
                    }
                }
                Event::Opened {
                    reload,
                    path,
                    document,
                    mut lock,
                    stamp,
                } => {
                    if let Some(target) = &reload {
                        if self.reloading.get(&target.tab_id) != Some(target) {
                            continue;
                        }
                        self.reloading.remove(&target.tab_id);
                    }
                    let mut prepared = match prepare_open(
                        &self.tabs,
                        self.next_id,
                        reload.as_ref(),
                        &path,
                        document,
                    ) {
                        Ok(prepared) => prepared,
                        Err(e) => {
                            self.error = e;
                            if let Some(target) = &reload
                                && self.tabs.iter().any(|t| t.id == target.tab_id)
                            {
                                self.disk_changed = Some(target.tab_id);
                            }
                            continue;
                        }
                    };
                    if let PreparedOpen::Install { tab, replace, .. } = &mut prepared {
                        if let Some((_, old_id)) = replace {
                            // Transfer the existing lock only after every completion guard passed.
                            lock = self.locks.remove(old_id).or(lock);
                            self.retire_state(*old_id);
                        }
                        tab.save_locked = lock.is_none();
                        if let Some(lock) = lock {
                            self.locks.insert(tab.id, lock);
                        }
                        self.stamps.insert(tab.id, stamp);
                    } else {
                        let id = commit_open(&mut self.tabs, &mut self.next_id, prepared);
                        self.deferred.push((id, "activate_external"));
                        continue;
                    }
                    let id = commit_open(&mut self.tabs, &mut self.next_id, prepared);
                    self.deferred.push((id, "activate_external"));
                    self.settings.remember(path);
                    self.status = if reload.is_some() {
                        "Файл перезагружен"
                    } else {
                        "Файл открыт"
                    }
                    .into();
                }
                Event::ReloadFailed { target, message } => {
                    if self.reloading.get(&target.tab_id) == Some(&target) {
                        self.reloading.remove(&target.tab_id);
                        self.error = message;
                        self.disk_changed = Some(target.tab_id);
                    }
                }
                Event::Saved {
                    id,
                    version,
                    path,
                    copy,
                    export,
                    stamp,
                    lock,
                } => {
                    self.saving.remove(&id);
                    if !copy
                        && !export
                        && let Some(t) = self.tabs.iter_mut().find(|t| t.id == id)
                    {
                        t.saved_version = version;
                        t.path = Some(path.clone());
                        t.title = path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        self.stamps.insert(id, stamp);
                        if let Some(l) = lock {
                            self.locks.insert(id, l);
                            t.save_locked = false;
                        }
                        if !t.dirty() {
                            self.recovery.clear(id);
                        }
                    }
                    self.settings.remember(path);
                    self.status = "Сохранено".into();
                    if let Some(reply) = self.mcp_saves.remove(&id) {
                        let _ = reply.send(Ok(json!({"applied":true,"version":version})));
                    }
                    if self.close_after_save == Some(id)
                        && self
                            .tabs
                            .iter()
                            .find(|t| t.id == id)
                            .is_some_and(|t| !t.dirty())
                    {
                        self.close_after_save = None;
                        self.remove(id);
                    }
                    if self.close_after_save == Some(id) {
                        self.close_after_save = None;
                        self.exit_after_save = false;
                        self.pending = Some(Pending::Close(id));
                        self.status = "После сохранения появились новые правки".into();
                    }
                }
                Event::Canceled { id } => {
                    if id == 0 {
                        self.opening = false;
                    }
                    self.picking.remove(&id);
                    self.close_after_save = None;
                    self.exit_after_save = false;
                    self.saving.remove(&id);
                    if let Some(reply) = self.mcp_saves.remove(&id) {
                        let _ = reply.send(Err("Отменено".into()));
                    }
                }
                Event::Error { message, id } => {
                    self.close_after_save = None;
                    self.exit_after_save = false;
                    if let Some(id) = id {
                        self.picking.remove(&id);
                        self.saving.remove(&id);
                    }
                    self.error = message.clone();
                    if let Some(id) = id
                        && let Some(reply) = self.mcp_saves.remove(&id)
                    {
                        let _ = reply.send(Err(message));
                    }
                }
                Event::Watched(stamps) => {
                    for (id, new) in stamps {
                        if !self.reloading.contains_key(&id)
                            && self.stamps.get(&id).is_some_and(|old| *old != new)
                        {
                            self.disk_changed = Some(id);
                        }
                    }
                }
            }
        }
    }
    /// Shows the black AI caret where an agent's Docs edit landed.
    fn mark_ai_caret(&self, ctx: &egui::Context, target: u64, args: &Value) {
        if !self
            .tabs
            .iter()
            .any(|t| t.id == target && matches!(t.document, Document::Docs(_)))
        {
            return;
        }
        let int = |keys: &[&str]| {
            keys.iter()
                .find_map(|k| args.get(*k).and_then(Value::as_u64))
        };
        let Some(paragraph_id) = int(&["paragraph_id", "end_paragraph_id", "start_paragraph_id"])
        else {
            return;
        };
        let from = int(&["offset", "start_offset"]).unwrap_or_default() as usize;
        let to = match (
            args.get("text").and_then(Value::as_str),
            int(&["end_offset"]),
        ) {
            (Some(text), _) => from.saturating_add(text.chars().count()),
            (None, Some(end)) => end as usize,
            _ => from,
        };
        let caret = crate::docs::editor::AiCaret {
            tab: target,
            paragraph_id,
            from,
            to,
            time: ctx.input(|i| i.time),
        };
        ctx.data_mut(|d| d.insert_temp(crate::docs::editor::ai_caret_id(), caret));
        ctx.request_repaint();
    }
    fn mcp_requests(&mut self, ctx: &egui::Context) {
        // Agents often send bursts of small edits; drain them in one frame.
        let started = Instant::now();
        for _ in 0..64 {
            if started.elapsed() >= std::time::Duration::from_millis(4) {
                ctx.request_repaint();
                break;
            }
            let Some(r) = self.bridge.as_ref().and_then(|b| b.try_recv().ok()) else {
                break;
            };
            if r.reply.is_closed() || Instant::now() > r.deadline {
                continue;
            }
            if !self.settings.mcp {
                let _ = r.reply.send(Err("permission_denied: MCP отключён".into()));
                continue;
            }
            if !self.allowed.contains(&r.client) && !self.chat.owns(&r.client) {
                self.clients.insert(r.client);
                let _ = r.reply.send(Err(
                    "needs_confirmation: разрешите клиент в окне TrineDocs".into()
                ));
                continue;
            }
            if !crate::mcp::tools::read_only(&r.tool) && self.pending.is_some() {
                let _ = r
                    .reply
                    .send(Err("busy: подтвердите действие в интерфейсе".into()));
                continue;
            }
            if r.args
                .get("mode")
                .is_some_and(|v| !matches!(v.as_str(), Some("apply" | "dry_run")))
                || r.args
                    .get("base_version")
                    .is_some_and(|v| v.as_u64().is_none())
            {
                let _ = r.reply.send(Err("invalid_args: mode/base_version".into()));
                continue;
            }
            if !crate::mcp::tools::names().contains(&r.tool.as_str()) {
                let _ = r
                    .reply
                    .send(Err("unsupported_feature: неизвестный инструмент MCP".into()));
                continue;
            }
            let dry_run = r.args.get("mode").and_then(Value::as_str) == Some("dry_run");
            let id = r.args.get("document_id").and_then(Value::as_u64);
            if id.is_some_and(|id| !self.tabs.iter().any(|t| t.id == id)) {
                let _ = r.reply.send(Err("not_found: документ не открыт".into()));
                continue;
            }
            if !crate::mcp::tools::read_only(&r.tool)
                && !dry_run
                && let Some(target) = id.or(self.active)
                && let Err(e) = self.flush_pending(target)
            {
                let _ = r.reply.send(Err(e));
                continue;
            }
            if id
                .and_then(|id| self.tabs.iter().find(|t| t.id == id))
                .is_some_and(|t| {
                    r.args
                        .get("base_version")
                        .and_then(Value::as_u64)
                        .is_some_and(|v| v != t.version)
                })
            {
                let _ = r
                    .reply
                    .send(Err("conflict: версия документа изменилась".into()));
                continue;
            }
            if matches!(
                r.tool.as_str(),
                "save_document" | "close_document" | "open_document" | "export"
            ) && dry_run
            {
                let _ = r.reply.send(Ok(json!({"applied":false})));
                continue;
            }
            if r.tool == "save_document" {
                if id.is_some_and(|id| self.reloading.contains_key(&id)) {
                    let _ = r
                        .reply
                        .send(Err("busy: дождитесь завершения перезагрузки".into()));
                    continue;
                }
                if let Some(id) = id
                    && self
                        .tabs
                        .iter()
                        .any(|t| t.id == id && t.path.is_some() && !t.read_only && !t.save_locked)
                    && !self.picking.contains(&id)
                    && !self.saving.contains(&id)
                    && !self.reloading.contains_key(&id)
                    && !self.mcp_saves.contains_key(&id)
                {
                    self.mcp_saves.insert(id, r.reply);
                    self.save(id, false, false, false, None);
                } else {
                    let message = match id.and_then(|id| self.tabs.iter().find(|t| t.id == id)) {
                        Some(t) if t.save_locked => {
                            "file_locked: правки доступны; сохраните копию или снимите блокировку в интерфейсе"
                        }
                        Some(t) if t.read_only => "read_only: документ защищён от правки",
                        Some(t)
                            if self.picking.contains(&t.id)
                                || self.saving.contains(&t.id)
                                || self.reloading.contains_key(&t.id)
                                || self.mcp_saves.contains_key(&t.id) =>
                        {
                            "busy: дождитесь файловой операции"
                        }
                        Some(_) => {
                            "needs_path: для первого сохранения выберите путь через интерфейс"
                        }
                        None => "not_found: требуется document_id",
                    };
                    let _ = r.reply.send(Err(message.into()));
                }
                continue;
            }
            if r.tool == "open_document" || r.tool == "export" {
                let _ = r.reply.send(Err(
                    "needs_confirmation: открытие пути и экспорт выполняются через меню Файл"
                        .into(),
                ));
                continue;
            }
            if r.tool == "close_document" {
                if id.is_some() {
                    self.pending_mcp_closes.push(r);
                } else {
                    let _ = r.reply.send(Err("Требуется document_id".into()));
                }
                continue;
            }
            let result = crate::mcp::tools::dispatch(
                &mut self.tabs,
                &mut self.next_id,
                &r.tool,
                &r.args,
                &r.client,
            );
            let applied_target = result
                .as_ref()
                .ok()
                .filter(|value| value.get("applied").and_then(Value::as_bool) == Some(true))
                .and_then(|value| id.or_else(|| value.get("document_id").and_then(Value::as_u64)));
            if let Some(target) = applied_target {
                self.last_agent = Some((target, r.client.clone(), Instant::now()));
                self.mark_ai_caret(ctx, target, &r.args);
                if r.tool == "create_document" {
                    self.deferred.push((target, "activate_external"));
                }
            }
            let busy_docs_target = result.as_ref().err().is_some_and(|e| {
                e.starts_with("conflict: дождитесь отображения")
                    || e.starts_with("too_large: очередь позиционных")
            });
            if busy_docs_target && let Some(id) = id {
                self.deferred.push((id, "activate_external"));
            }
            let needs_docs_reconcile = busy_docs_target
                || (result.is_ok()
                    && id.is_some_and(|target| {
                        self.tabs.iter().any(|t| {
                            t.id == target
                                && matches!(&t.document, Document::Docs(_))
                                && t.pending_docs_edits.is_none()
                        })
                    }));
            let _ = r.reply.send(result);
            if needs_docs_reconcile {
                ctx.request_repaint();
                break;
            }
        }
    }
    fn finish_mcp_closes(&mut self) {
        for request in std::mem::take(&mut self.pending_mcp_closes) {
            if request.reply.is_closed() || Instant::now() > request.deadline {
                continue;
            }
            let result = (|| -> Result<Value, String> {
                if !self.settings.mcp
                    || (!self.allowed.contains(&request.client) && !self.chat.owns(&request.client))
                {
                    return Err("permission_denied: доступ MCP отозван".into());
                }
                let id = request
                    .args
                    .get("document_id")
                    .and_then(Value::as_u64)
                    .ok_or("Требуется document_id")?;
                if self.pending.is_some() {
                    return Err("busy: подтвердите действие в интерфейсе".into());
                }
                self.flush_pending(id)?;
                let tab = self
                    .tabs
                    .iter()
                    .find(|tab| tab.id == id)
                    .ok_or("not_found: документ не открыт")?;
                if request
                    .args
                    .get("base_version")
                    .and_then(Value::as_u64)
                    .is_some_and(|version| version != tab.version)
                {
                    return Err("conflict: версия документа изменилась".into());
                }
                if self.picking.contains(&id)
                    || self.saving.contains(&id)
                    || self.reloading.contains_key(&id)
                {
                    return Err("busy: дождитесь файловой операции".into());
                }
                self.close(id);
                Ok(json!({
                    "applied": !self.tabs.iter().any(|tab| tab.id == id),
                    "needs_confirmation": matches!(self.pending, Some(Pending::Close(target)) if target == id)
                }))
            })();
            let _ = request.reply.send(result);
        }
    }
    fn tick(&mut self, ctx: &egui::Context) {
        self.chat.poll();
        let previous_active = self.active;
        self.mcp_saves.retain(|_, reply| !reply.is_closed());
        if self.exit_after_save && self.close_after_save.is_none() && self.pending.is_none() {
            if let Some(id) = self.tabs.iter().find(|t| t.dirty()).map(|t| t.id) {
                self.close_after_save = Some(id);
                self.save(id, false, false, false, None);
            } else {
                self.allow_exit = true;
                self.exit_after_save = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        self.mcp_requests(ctx);
        if self.active != previous_active {
            crate::slides::editor::stop_presentation(ctx);
        }
        if let Some(e) = self.recovery.error() {
            self.status = format!("Ошибка восстановления: {e}");
        }
        if self.last_snapshot.elapsed() >= crate::recovery::INTERVAL {
            for t in &self.tabs {
                if t.dirty()
                    && self.snapshot_versions.get(&t.id) != Some(&t.version)
                    && self.recovery.snapshot(crate::recovery::Snapshot {
                        id: t.id,
                        title: t.title.clone(),
                        path: t.path.clone(),
                        version: t.version,
                        document: t.document.clone(),
                    })
                {
                    self.snapshot_versions.insert(t.id, t.version);
                }
            }
            self.last_snapshot = Instant::now();
        }
        if self.last_watch.elapsed().as_secs() > 2 {
            let paths = self
                .tabs
                .iter()
                .filter_map(|t| t.path.clone().map(|p| (t.id, p)))
                .collect();
            let _ = self.worker.send(Job::Watch(paths));
            self.last_watch = Instant::now();
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.pending.is_some() {
            return;
        }
        if crate::slides::editor::is_presenting(ctx) {
            return;
        }
        let cmd = egui::Modifiers::COMMAND;
        if ctx.input_mut(|i| i.consume_key(cmd | egui::Modifiers::SHIFT, egui::Key::A)) {
            self.show_ai = !self.show_ai;
        }
        if ctx.input_mut(|i| i.consume_key(cmd, egui::Key::O)) {
            self.send(Job::PickOpen);
        }
        if ctx.input_mut(|i| i.consume_key(cmd, egui::Key::S))
            && let Some(id) = self.active
        {
            self.deferred.push((id, "save"));
        }
        if ctx.input_mut(|i| i.consume_key(cmd, egui::Key::W))
            && let Some(id) = self.active
        {
            self.deferred.push((id, "close"));
        }
        let palette_boundary = ctx.input(|i| i.events.iter().position(|e| matches!(e, egui::Event::Key { key: egui::Key::K, pressed: true, modifiers, .. } if modifiers.command || modifiers.ctrl)));
        if let Some(boundary) = palette_boundary {
            if self.palette {
                ctx.input_mut(|i| i.consume_key(cmd, egui::Key::K));
                self.palette = false;
                ctx.memory_mut(|m| m.surrender_focus(egui::Id::new("command-palette-query")));
            } else {
                self.queue_palette(ctx, Some(boundary));
            }
        }
        if self.palette
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.palette = false;
        }
        if !ctx.text_edit_focused() {
            for (key, action) in [(egui::Key::Z, "undo"), (egui::Key::Y, "redo")] {
                if ctx.input_mut(|i| i.consume_key(cmd, key))
                    && let Some(id) = self.active
                {
                    self.deferred.push((id, action));
                }
            }
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Tab))
            && !self.tabs.is_empty()
        {
            let index = self
                .tabs
                .iter()
                .position(|t| Some(t.id) == self.active)
                .unwrap_or(self.tabs.len() - 1);
            if let Some(id) = self.tabs.get((index + 1) % self.tabs.len()).map(|t| t.id) {
                self.deferred.push((id, "activate"));
            }
        }
        for f in ctx.input(|i| i.raw.dropped_files.clone()) {
            self.send(Job::Open(f.path().to_path_buf()));
        }
    }
    fn home(&mut self, ui: &mut egui::Ui) {
        let filter_id = egui::Id::new("home-filter");
        let mut filter: usize = ui.data(|d| d.get_temp(filter_id)).unwrap_or_default();
        egui::ScrollArea::vertical().id_salt("home").show(ui, |ui| {
            let width = ui.available_width().min(960.0);
            let inset = ((ui.available_width() - width) * 0.5).max(0.0);
            ui.horizontal(|ui| {
                ui.add_space(inset);
                ui.vertical(|ui| {
                    ui.set_width(width);
                    ui.add_space(28.0);
                    ui.label(egui::RichText::new("Создать").size(20.0).strong());
                    ui.add_space(12.0);
                    let cards = [
                        ("docs", "Новый документ", egui::Color32::from_rgb(43, 87, 154)),
                        ("sheets", "Новая таблица", egui::Color32::from_rgb(33, 115, 70)),
                        ("slides", "Новая презентация", egui::Color32::from_rgb(196, 62, 28)),
                    ];
                    let gap = 16.0;
                    let card_width = ((width - gap * 2.0) / 3.0).clamp(120.0, 220.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
                        for (kind, label, color) in cards {
                            if new_card(ui, kind, label, color, card_width).clicked() {
                                self.create(kind);
                            }
                        }
                    });
                    ui.add_space(32.0);
                    ui.horizontal_wrapped(|ui| {
                        for (index, label) in ["Недавние", "Закреплённые"].into_iter().enumerate() {
                            let selected = filter % 2 == index;
                            let text = egui::RichText::new(label).size(15.0);
                            let response = ui.add(
                                egui::Button::new(if selected { text.strong() } else { text.color(crate::chrome::muted(ui)) })
                                    .frame_when_inactive(false)
                                    .min_size(egui::vec2(0.0, 30.0)),
                            );
                            if selected {
                                ui.painter().line_segment(
                                    [response.rect.left_bottom() + egui::vec2(8.0, -1.0), response.rect.right_bottom() - egui::vec2(8.0, 1.0)],
                                    egui::Stroke::new(2.0, crate::chrome::accent(ui)),
                                );
                            }
                            if response.clicked() {
                                filter = index;
                            }
                        }
                        ui.add(
                            egui::TextEdit::singleline(&mut self.query)
                                .hint_text("Поиск файлов")
                                .desired_width((width - 380.0).clamp(120.0, 220.0))
                                .margin(egui::Margin::symmetric(10, 6)),
                        );
                        if crate::chrome::icon_button(ui, crate::chrome::Icon::Open, "Открыть файл · Ctrl+O", true).clicked() {
                            self.send(Job::PickOpen);
                        }
                    });
                    ui.add_space(6.0);
                    let list: Vec<PathBuf> = if filter % 2 == 1 { self.settings.pinned.clone() } else { self.settings.recent.clone() };
                    let q = self.query.trim().to_lowercase();
                    let shown: Vec<&PathBuf> = list.iter().filter(|p| p.to_string_lossy().to_lowercase().contains(&q)).collect();
                    if shown.is_empty() {
                        ui.add_space(24.0);
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new(if !q.is_empty() { "Ничего не найдено" } else if filter % 2 == 1 { "Нет закреплённых файлов" } else { "Здесь появятся ваши файлы" }).strong());
                            ui.label(egui::RichText::new(if !q.is_empty() { "Попробуйте другое имя или путь." } else if filter % 2 == 1 { "Нажмите ☆ рядом с файлом, чтобы закрепить его." } else { "Создайте файл или перетащите его в окно." }).color(crate::chrome::muted(ui)));
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.add_space(52.0);
                            ui.label(egui::RichText::new("Имя").size(11.5).color(crate::chrome::muted(ui)));
                        });
                        ui.add_space(2.0);
                    }
                    for p in shown {
                        let pin = self.settings.pinned.contains(p);
                        let row = ui.allocate_ui(egui::vec2(width, 48.0), |ui| {
                            let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 48.0), egui::Sense::click());
                            response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), p.display().to_string()));
                            let hovered = ui.rect_contains_pointer(rect);
                            if hovered {
                                ui.painter().rect_filled(rect, 6.0, crate::chrome::hover_fill(ui));
                            }
                            let (glyph, color) = file_badge(p);
                            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(egui::vec2(10.0, 6.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
                            crate::chrome::badge(&mut child, glyph, color, 28.0);
                            child.add_space(10.0);
                            child.vertical(|ui| {
                                ui.set_width((width - 122.0).max(40.0));
                                ui.spacing_mut().item_spacing.y = 0.0;
                                ui.add(egui::Label::new(egui::RichText::new(p.file_name().map(|s| s.to_string_lossy()).unwrap_or_default()).size(13.5)).truncate());
                                ui.add(egui::Label::new(egui::RichText::new(p.parent().map(|d| d.display().to_string()).unwrap_or_default()).color(crate::chrome::muted(ui)).size(11.0)).truncate());
                            });
                            let mut action = None;
                            child.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if hovered || pin {
                                    if hovered && crate::chrome::tool(ui, "×", "Удалить из списка", false).clicked() {
                                        action = Some("remove");
                                    }
                                    if crate::chrome::tool(ui, if pin { "★" } else { "☆" }, if pin { "Открепить" } else { "Закрепить" }, false).clicked() {
                                        action = Some("pin");
                                    }
                                }
                            });
                            (response, action)
                        });
                        let (response, action) = row.inner;
                        match action {
                            Some("pin") => {
                                if pin { self.settings.pinned.retain(|v| v != p); } else { self.settings.pinned.push(p.clone()); }
                                if let Err(e) = self.settings.save() { self.error = e; }
                            }
                            Some(_) => {
                                self.settings.recent.retain(|v| v != p);
                                self.settings.pinned.retain(|v| v != p);
                                if let Err(e) = self.settings.save() { self.error = e; }
                            }
                            None => {
                                if response.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(p.display().to_string()).clicked() {
                                    self.send(Job::Open(p.clone()));
                                }
                            }
                        }
                    }
                    ui.add_space(28.0);
                    ui.label(egui::RichText::new(format!("TrineDocs {} · поддержка всех возможностей Microsoft Office пока неполная.", env!("CARGO_PKG_VERSION"))).color(crate::chrome::muted(ui)).size(11.0));
                });
            });
        });
        ui.data_mut(|d| d.insert_temp(filter_id, filter));
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if !self.error.is_empty() {
            let error = self.error.clone();
            egui::Window::new("Сообщение")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(error);
                    if ui.button("OK").clicked() {
                        self.error.clear();
                    }
                });
        }
        if let Some(client) = self.clients.first().cloned() {
            egui::Window::new("Разрешить MCP-клиент?")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(format!("Клиент: {client}"));
                    ui.label(
                        "Чтение и правка открытых документов. Файлы на диске — через интерфейс.",
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Разрешить").clicked() {
                            self.allowed.insert(client.clone());
                            self.clients.remove(&client);
                        }
                        if ui.button("Запретить").clicked() {
                            self.clients.remove(&client);
                        }
                    });
                });
        }
        if !self.recoveries.is_empty() {
            egui::Window::new("Восстановление")
                .collapsible(false)
                .show(ctx, |ui| {
                    for s in self.recoveries.clone() {
                        ui.horizontal(|ui| {
                            ui.label(&s.title);
                            if ui.button("Восстановить").clicked() {
                                let (id, following) = match fresh_tab_id(&self.tabs, self.next_id) {
                                    Ok(ids) => ids,
                                    Err(e) => {
                                        self.error = e;
                                        return;
                                    }
                                };
                                let mut t = Tab::new(id, s.document.clone(), s.title.clone(), None);
                                t.saved_version = u64::MAX;
                                if !self.recovery.restore(s.id, crate::recovery::Snapshot {
                                    id,
                                    title: t.title.clone(),
                                    path: None,
                                    version: t.version,
                                    document: t.document.clone(),
                                }) {
                                    self.error = "Не удалось поставить восстановление в очередь; исходный снимок сохранён.".into();
                                    return;
                                }
                                self.next_id = following;
                                self.snapshot_versions.insert(id, t.version);
                                self.tabs.push(t);
                                self.active = Some(id);
                                self.recoveries.retain(|r| r.id != s.id);
                            }
                        });
                    }
                    if ui.button("Удалить снимки").clicked() {
                        for s in &self.recoveries {
                            self.recovery.clear(s.id);
                        }
                        self.recoveries.clear();
                    }
                });
        }
        if let Some(id) = self.disk_changed {
            egui::Window::new("Файл изменён на диске").collapsible(false).show(ctx,|ui|{
            ui.label("Перезагрузка заменит текущие правки. Для их сохранения используйте «Сохранить копию».");ui.horizontal(|ui|{
                if ui.button("Перезагрузить").clicked() { self.reload(id); }
                if ui.button("Оставить правки").clicked(){self.stamps.insert(id, self.tabs.iter().find(|t|t.id==id).and_then(|t|t.path.as_ref()).and_then(|p|crate::file_io::stamp(p)));self.disk_changed=None;}
            });
        });
        }
        if self.pending.is_some() {
            let mut action = 0;
            egui::Modal::new(egui::Id::new("confirmation")).show(ctx,|ui|{
            ui.set_max_width(520.0);
            ui.heading("Подтверждение");
            ui.add_space(8.0);
            match self.pending.as_ref(){Some(Pending::Unlock{path,marker,..})=>{
                ui.label(format!("Файл: {}", path.file_name().map(|v| v.to_string_lossy()).unwrap_or_default()));
                ui.label(if marker.is_some() { "Удалить оставшийся lock-маркер? Закройте этот файл в других программах. Активная блокировка TrineDocs остаётся защищённой." } else { "Lock-маркер уже отсутствует. Получить блокировку для сохранения?" });
                ui.horizontal(|ui| { if ui.button("Разблокировать сохранение").clicked(){action=1}
                if ui.button("Отмена").clicked(){action=3} });
            },Some(Pending::Register(p))=>{ui.label(format!("Добавить сервер trinedocs в {}",p.path.display()));ui.label("Остальные записи сохранятся. Существующая запись trinedocs не заменяется.");ui.monospace(&p.change);ui.horizontal(|ui|{if ui.button("Добавить").clicked(){action=1}
if ui.button("Отмена").clicked(){action=3}});},Some(Pending::SaveFormat{id,..})=>{if let Some(t)=self.tabs.iter().find(|t|t.id==*id){match &t.document{Document::Docs(d)=>{for loss in &d.losses{ui.label(loss);}},Document::Slides(d)=>{for loss in d.export_warnings(){ui.label(loss);}},_=>{}}}ui.label("Экспорт сохраняет только возможности выбранного формата. Форматирование и объекты могут измениться или исчезнуть. Для редактируемой копии выберите .docx, .xlsx или .pptx; ограничения Office-экспорта также остаются.");ui.horizontal(|ui|{if ui.button("Продолжить").clicked(){action=1}
if ui.button("Отмена").clicked(){action=3}});}
            _=>{ui.label("Сохранить изменения?");ui.horizontal(|ui|{if ui.button("Сохранить").clicked(){action=1}
if ui.button("Не сохранять").clicked(){action=2}
if ui.button("Отмена").clicked(){action=3}});}}
        });
            if action != 0
                && let Some(p) = self.pending.take()
            {
                match (p.clone(), action) {
                    (Pending::Unlock { id, path, marker }, 1) => {
                        if !self.saving.contains(&id)
                            && !self.reloading.contains_key(&id)
                            && self
                                .tabs
                                .iter()
                                .any(|t| t.id == id && t.path.as_ref() == Some(&path))
                        {
                            self.send(Job::Unlock {
                                id,
                                path,
                                marker,
                                expected: self.stamps.get(&id).copied().flatten(),
                            });
                        } else {
                            self.error = "Документ изменился или занят файловой операцией".into();
                        }
                    }
                    (Pending::Register(p), 1) => self.send(Job::ApplyRegistration(p)),
                    (
                        Pending::SaveFormat {
                            id,
                            path,
                            copy,
                            export,
                        },
                        1,
                    ) => self.write(id, path, copy, export, false),
                    (Pending::Close(id), 1) => {
                        self.close_after_save = Some(id);
                        self.save(id, false, false, false, None);
                    }
                    (Pending::Close(id), 2) => self.remove(id),
                    (Pending::Exit, 1) => {
                        self.exit_after_save = true;
                        if let Some(id) = self.tabs.iter().find(|t| t.dirty()).map(|t| t.id) {
                            self.close_after_save = Some(id);
                            self.save(id, false, false, false, None);
                        }
                    }
                    (Pending::Exit, 2) => {
                        self.allow_exit = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    _ => {
                        let id = match &p {
                            Pending::SaveFormat { id, .. }
                            | Pending::Unlock { id, .. }
                            | Pending::Close(id) => Some(*id),
                            _ => self.close_after_save,
                        };
                        if let Some(id) = id
                            && let Some(reply) = self.mcp_saves.remove(&id)
                        {
                            let _ = reply.send(Err("Отменено в интерфейсе".into()));
                        }
                        self.close_after_save = None;
                        self.exit_after_save = false;
                    }
                }
            }
        }
        if self.show_settings {
            let mut open = true;
            let mut changed = false;
            egui::Window::new("Настройки")
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Язык");
                        changed |= ui
                            .selectable_value(&mut self.settings.language, "ru".into(), "Русский")
                            .changed();
                        changed |= ui
                            .selectable_value(&mut self.settings.language, "en".into(), "English")
                            .changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Тема");
                        for (v, l) in [
                            ("system", "Системная"),
                            ("light", "Светлая"),
                            ("dark", "Тёмная"),
                        ] {
                            changed |= ui
                                .selectable_value(&mut self.settings.theme, v.into(), l)
                                .changed();
                        }
                    });
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut self.settings.scale, 0.75..=2.0).text("Масштаб"),
                        )
                        .changed();
                    changed |= ui
                        .checkbox(&mut self.settings.backup, "Резервная копия .bak")
                        .changed();
                    changed |= ui
                        .checkbox(&mut self.settings.reduced_motion, "Уменьшить движение")
                        .changed();
                    if ui
                        .checkbox(&mut self.settings.mcp, "MCP: локальные клиенты")
                        .changed()
                    {
                        changed = true;
                        self.allowed.clear();
                        self.clients.clear();
                        if self.settings.mcp && self.mcp.is_none() {
                            let (m, b) = crate::mcp::Handle::start(ctx.clone());
                            self.mcp = Some(m);
                            self.bridge = Some(b);
                        }
                    }
                    if let Some(h) = &self.mcp
                        && let Ok(state) = h.connection.lock()
                    {
                        match &*state {
                            Ok(c) => {
                                ui.label(format!("127.0.0.1:{} · Bearer token", c.port));
                            }
                            Err(e) => {
                                ui.label(e);
                            }
                        }
                    }
                    if ui.button("Скопировать конфиг MCP").clicked() {
                        let shim = std::env::current_exe()
                            .ok()
                            .map(|p| p.with_file_name("trinedocs-mcp.exe"))
                            .unwrap_or_default();
                        ctx.copy_text(
                            json!({"mcpServers":{"trinedocs":{"command":shim,"args":[]}}})
                                .to_string(),
                        );
                    }
                    ui.label("Регистрация MCP с предпросмотром:");
                    for client in [
                        crate::mcp::register::Client::Codex,
                        crate::mcp::register::Client::Claude,
                        crate::mcp::register::Client::Cursor,
                        crate::mcp::register::Client::VsCode,
                    ] {
                        if ui.button(client.label()).clicked() {
                            self.send(Job::PreviewRegistration(client));
                        }
                    }
                });
            self.show_settings = open;
            if changed {
                ctx.set_zoom_factor(self.settings.scale);
                if let Err(e) = self.settings.save() {
                    self.error = e
                }
            }
        }
        if self.palette {
            let mut open = true;
            egui::Window::new("Команды")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(440.0)
                .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 96.0))
                .show(ctx, |ui| {
                    let query_id = egui::Id::new("command-palette-query");
                    let enter = ctx.memory(|m| m.focused() == Some(query_id))
                        && ctx
                            .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.palette_query)
                            .id(query_id)
                            .hint_text("Найти действие…")
                            .desired_width(400.0),
                    );
                    if self.palette_focus {
                        response.request_focus();
                        self.palette_focus = false;
                    }
                    let commands = palette_commands(self.current().is_some(), &self.palette_query);
                    let mut picked = None;
                    for (index, &(key, label)) in commands.iter().enumerate() {
                        if ui
                            .add_sized(
                                [ui.available_width(), 28.0],
                                egui::Button::new(label).frame_when_inactive(false),
                            )
                            .clicked()
                            || (enter && index == 0)
                        {
                            picked = Some(key);
                        }
                    }
                    if commands.is_empty() {
                        ui.weak("Команды не найдены");
                    }
                    if let Some(key) = picked {
                        match key {
                            "docs" | "sheets" | "slides" => self.create(key),
                            "open" => self.send(Job::PickOpen),
                            "settings" => self.show_settings = true,
                            "close" => {
                                if let Some(id) = self.active {
                                    self.close(id);
                                }
                            }
                            _ => {
                                if let Some(id) = self.active {
                                    self.save(
                                        id,
                                        key == "copy",
                                        key == "pdf",
                                        key == "saveas",
                                        if key == "pdf" { Some("pdf") } else { None },
                                    );
                                }
                            }
                        }
                        self.palette = false;
                        ctx.memory_mut(|m| m.surrender_focus(response.id));
                    }
                });
            self.palette &= open;
        }
    }
}
impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        crate::clipboard::observe(ctx);
        self.tick(ctx);
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Some(smoke) = &mut self.smoke {
            self.allow_exit = true;
            match if self.error.is_empty() {
                smoke.tick(&ctx)
            } else {
                Err(self.error.clone())
            } {
                Ok(Some(document)) => {
                    self.tabs.clear();
                    let mut t =
                        Tab::new(self.next_id, document, "Проверка интерфейса".into(), None);
                    self.next_id += 1;
                    t.saved_version = 0;
                    self.active = Some(t.id);
                    self.tabs.push(t);
                }
                Ok(None) => {}
                Err(e) => {
                    self.error = e;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        let previous_active = self.active;
        self.shortcuts(&ctx);
        ctx.set_theme(match self.settings.theme.as_str() {
            "dark" => egui::ThemePreference::Dark,
            "light" => egui::ThemePreference::Light,
            _ => egui::ThemePreference::System,
        });
        let kind = self.current().map(|t| t.document.kind()).unwrap_or("docs");
        crate::chrome::style(&ctx, kind, self.settings.reduced_motion);
        ui.set_style(ctx.global_style());
        let mut command = None;
        let mut reject_ai = None;
        let mut external_activation = None;
        let mut user_activation = false;
        if self.pending.is_some() || self.palette {
            ui.disable();
        }
        let compact_header = ui.available_width() < 1050.0;
        egui::Panel::top("shell")
            .frame(
                egui::Frame::new()
                    .fill(ui.visuals().window_fill)
                    .inner_margin(egui::Margin::symmetric(14, 7)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    crate::chrome::badge(ui, "T", crate::chrome::accent(ui), 28.0);
                    if !compact_header {
                        ui.label(egui::RichText::new("TrineDocs").size(15.0).strong());
                    }
                    ui.add_space(8.0);
                    ui.menu_button(self.tr("File"), |ui| {
                        for (c, l) in [
                            ("docs", "Document"),
                            ("sheets", "Spreadsheet"),
                            ("slides", "Presentation"),
                            ("open", "Open"),
                            ("save", "Save"),
                            ("saveas", "Save as"),
                            ("copy", "Save copy"),
                            ("pdf", "Export PDF"),
                            ("print", "Print via PDF viewer"),
                            ("close", "Close"),
                        ] {
                            if ui
                                .add_enabled(
                                    matches!(c, "docs" | "sheets" | "slides" | "open")
                                        || self.active.is_some(),
                                    egui::Button::new(self.tr(l)),
                                )
                                .clicked()
                            {
                                command = Some(c);
                                ui.close();
                            }
                        }
                        if let Some(t) = self.current() {
                            ui.separator();
                            ui.menu_button(
                                "Сохранить / экспортировать…",
                                |ui| {
                                    let formats: &[&str] = match &t.document {
                                        Document::Docs(_) => &["docx", "pdf", "txt", "md", "html"],
                                        Document::Sheets(_) => {
                                            &["xlsx", "pdf", "csv", "tsv", "html"]
                                        }
                                        Document::Slides(_) => &["pptx", "pdf", "png"],
                                    };
                                    for ext in formats {
                                        let label = t
                                            .document
                                            .save_formats()
                                            .iter()
                                            .find(|(e, _)| e == ext)
                                            .map(|(_, l)| *l)
                                            .unwrap_or(ext);
                                        if ui.button(format!("{label}  ·  .{ext}")).clicked() {
                                            command = Some(*ext);
                                            ui.close();
                                        }
                                    }
                                },
                            );
                        }
                    });
                    if crate::chrome::icon_button(
                        ui,
                        crate::chrome::Icon::Open,
                        "Открыть · Ctrl+O",
                        true,
                    )
                    .clicked()
                    {
                        command = Some("open")
                    }
                    if ui
                        .add_enabled(
                            self.active.is_some(),
                            egui::Button::new(self.tr("Save")).frame(false),
                        )
                        .clicked()
                    {
                        command = Some("save")
                    }
                    let can_undo = self.current().is_some_and(|t| !t.undo.is_empty());
                    let can_redo = self.current().is_some_and(|t| !t.redo.is_empty());
                    if crate::chrome::icon_button(
                        ui,
                        crate::chrome::Icon::Undo,
                        "Отменить · Ctrl+Z",
                        can_undo,
                    )
                    .clicked()
                    {
                        command = Some("undo");
                    }
                    if crate::chrome::icon_button(
                        ui,
                        crate::chrome::Icon::Redo,
                        "Повторить · Ctrl+Y",
                        can_redo,
                    )
                    .clicked()
                    {
                        command = Some("redo");
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.menu_button("···", |ui| {
                            if ui.button(self.tr("Settings")).clicked() {
                                self.show_settings = true;
                                ui.close();
                            }
                            if ui.button("Поиск команд · Ctrl+K").clicked() {
                                self.queue_palette_click(&ctx);
                                ui.close();
                            }
                        })
                        .response
                        .on_hover_text("Настройки и команды");
                        let dark = ui.visuals().dark_mode;
                        if crate::chrome::tool(
                            ui,
                            if dark { "☀" } else { "☾" },
                            if dark {
                                "Светлая тема"
                            } else {
                                "Тёмная тема"
                            },
                            false,
                        )
                        .clicked()
                        {
                            self.settings.theme = if dark { "light" } else { "dark" }.into();
                            if let Err(e) = self.settings.save() {
                                self.error = e;
                            }
                        }
                        if ui
                            .add(
                                egui::Button::new("AI")
                                    .selected(self.show_ai)
                                    .frame_when_inactive(false),
                            )
                            .on_hover_text("AI-чат и изменения (Ctrl+Shift+A)")
                            .clicked()
                        {
                            self.show_ai = !self.show_ai;
                        }
                        if compact_header {
                            if crate::chrome::icon_button(
                                ui,
                                crate::chrome::Icon::Search,
                                "Поиск команд · Ctrl+K",
                                true,
                            )
                            .clicked()
                            {
                                self.queue_palette_click(&ctx);
                            }
                        } else if ui
                            .add_sized(
                                [210.0, 28.0],
                                egui::Button::new(
                                    egui::RichText::new("Поиск команд    Ctrl+K")
                                        .color(crate::chrome::muted(ui)),
                                )
                                .fill(ui.visuals().panel_fill),
                            )
                            .clicked()
                        {
                            self.queue_palette_click(&ctx);
                        }
                    });
                });
                ui.add_space(6.0);
                egui::ScrollArea::horizontal()
                    .id_salt("document-tabs")
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            let home = doc_tab(
                                ui,
                                "⌂ Главная",
                                self.active.is_none(),
                                egui::Color32::TRANSPARENT,
                            );
                            #[cfg(test)]
                            self.shell_rects.insert((0, "home"), home.rect);
                            if home.clicked() {
                                self.deferred.push((0, "home"));
                            }
                            let mut closed = None;
                            for t in &self.tabs {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 0.0;
                                    let tab = doc_tab(
                                        ui,
                                        &format!(
                                            "{}{}",
                                            t.title,
                                            if t.dirty() { " •" } else { "" }
                                        ),
                                        self.active == Some(t.id),
                                        match &t.document {
                                            Document::Docs(_) => {
                                                egui::Color32::from_rgb(43, 87, 154)
                                            }
                                            Document::Sheets(_) => {
                                                egui::Color32::from_rgb(33, 115, 70)
                                            }
                                            Document::Slides(_) => {
                                                egui::Color32::from_rgb(196, 62, 28)
                                            }
                                        },
                                    );
                                    #[cfg(test)]
                                    self.shell_rects.insert((t.id, "activate"), tab.rect);
                                    if tab.clicked() {
                                        self.deferred.push((t.id, "activate"));
                                    }
                                    let close = crate::chrome::tool(ui, "×", "Закрыть", false);
                                    ui.add_space(6.0);
                                    #[cfg(test)]
                                    self.shell_rects.insert((t.id, "close"), close.rect);
                                    if close.clicked() {
                                        closed = Some(t.id)
                                    }
                                });
                            }
                            if let Some(id) = closed {
                                self.deferred.push((id, "close"));
                            }
                        });
                    });
            });
        egui::Panel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(ui.visuals().panel_fill)
                    .inner_margin(egui::Margin::symmetric(12, 5)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.small(&self.status);
                    if let Some(t) = self.current() {
                        ui.separator();
                        ui.small(match &t.document {
                            Document::Docs(_) => "Документ",
                            Document::Sheets(_) => "Таблица",
                            Document::Slides(_) => "Презентация",
                        });
                        if t.dirty() {
                            ui.label(
                                egui::RichText::new("● Не сохранено")
                                    .small()
                                    .color(ui.visuals().warn_fg_color),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new("Сохранено")
                                    .small()
                                    .color(crate::chrome::muted(ui)),
                            );
                        }
                        if t.save_locked {
                            ui.label(
                                egui::RichText::new("Файл занят · правки можно сохранить копией")
                                    .color(ui.visuals().warn_fg_color),
                            );
                        }
                    }
                    if let Some((id, name, time)) = &self.last_agent
                        && Some(*id) == self.active
                        && time.elapsed().as_secs_f32() < 1.5
                    {
                        ui.colored_label(
                            egui::Color32::from_rgb(156, 116, 237),
                            format!("AI · {name}"),
                        );
                        ctx.request_repaint();
                    }
                });
            });
        if let Some(id) = self.current().filter(|t| t.save_locked).map(|t| t.id) {
            egui::Panel::top("file-lock").frame(egui::Frame::new().fill(ui.visuals().window_fill).inner_margin(egui::Margin::symmetric(12, 7))).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Файл занят. Редактирование доступно; исходный файл защищён от перезаписи.");
                    if ui.button("Сохранить копию…").clicked() { command = Some("copy"); }
                    if ui.button("Блокировка…").clicked() { self.deferred.push((id, "unlock")); }
                });
            });
        }
        if self.show_ai {
            let pending_changes = self
                .tabs
                .iter()
                .find(|t| Some(t.id) == self.active)
                .map_or(0, |t| t.ai.iter().filter(|c| !c.accepted).count());
            let document = self
                .tabs
                .iter()
                .find(|t| Some(t.id) == self.active)
                .map(|t| crate::chat::Context {
                    id: t.id,
                    title: t.title.clone(),
                    kind: t.document.kind(),
                    selection: ctx
                        .data(|d| {
                            d.get_temp::<(Option<u64>, String)>(crate::docs::editor::selection_id())
                        })
                        .filter(|(tab, _)| *tab == Some(t.id))
                        .map(|(_, text)| text)
                        .unwrap_or_default(),
                });
            egui::Panel::right("ai-review")
                .default_size(340.0)
                .min_size(260.0)
                .frame(
                    egui::Frame::new()
                        .fill(ui.visuals().panel_fill)
                        .inner_margin(egui::Margin::symmetric(12, 10)),
                )
                .show(ui, |ui| {
                    let changes = if pending_changes > 0 {
                        format!("Изменения · {pending_changes}")
                    } else {
                        "Изменения".into()
                    };
                    crate::chrome::tabs(ui, &mut self.ai_tab, &["Чат", &changes]);
                    ui.add_space(6.0);
                    if self.ai_tab == 0 {
                        if self.chat.show(ui, document, self.settings.mcp) {
                            (self.settings.chat_models, self.settings.chat_efforts) =
                                self.chat.prefs();
                            if let Err(e) = self.settings.save() {
                                self.error = e;
                            }
                        }
                        return;
                    }
                    if let Some(t) = self.tabs.iter_mut().find(|t| Some(t.id) == self.active) {
                        let mut reject = None;
                        if pending_changes == 0 {
                            ui.add_space(16.0);
                            ui.label(
                                egui::RichText::new("Нет непринятых правок AI.")
                                    .color(crate::chrome::muted(ui)),
                            );
                        }
                        let mut accept_all = false;
                        if pending_changes > 1 && ui.button("Принять все").clicked() {
                            accept_all = true;
                        }
                        egui::ScrollArea::vertical()
                            .id_salt("ai-changes")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for c in t.ai.clone().iter().rev() {
                                    if !c.accepted {
                                        ui.add(egui::Label::new(&c.label).wrap());
                                        ui.horizontal(|ui| {
                                            if ui.button("Принять").clicked()
                                                && let Some(v) =
                                                    t.ai.iter_mut().find(|v| v.id == c.id)
                                            {
                                                v.accepted = true;
                                            }
                                            if ui.button("Откатить").clicked() {
                                                reject = Some(c.id)
                                            }
                                        });
                                        ui.separator();
                                    }
                                }
                            });
                        if accept_all {
                            for v in &mut t.ai {
                                v.accepted = true;
                            }
                        }
                        if let Some(id) = reject {
                            reject_ai = Some((t.id, id));
                        }
                    }
                });
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(crate::chrome::workspace(ui))
                    .inner_margin(12),
            )
            .show(ui, |ui| {
                if let Some(t) = self.tabs.iter_mut().find(|t| Some(t.id) == self.active) {
                    let mut d = t.document.clone();
                    ui.ctx().data_mut(|data| {
                        data.insert_temp(crate::docs::editor::active_tab_id(), t.id)
                    });
                    let docs_edits = t.pending_docs_edits.take();
                    t.pending_docs_edits = Some(Vec::new());
                    let changed = ui
                        .push_id(t.id, |ui| {
                            ui.add_enabled_ui(!t.read_only, |ui| match &mut d {
                                Document::Docs(d) => crate::docs::editor::show_with_edits(
                                    ui,
                                    d,
                                    docs_edits.as_deref(),
                                ),
                                Document::Sheets(b) => {
                                    crate::sheets::editor::show_for_tab(ui, b, t.id)
                                }
                                Document::Slides(d) => crate::slides::editor::show(ui, d),
                            })
                            .inner
                        })
                        .inner;
                    if changed {
                        if let Err(e) = t.apply(Op::Replace {
                            document: d,
                            label: "Редактирование".into(),
                            author: Author::User,
                            base_version: None,
                        }) {
                            self.error = e
                        } else if matches!(&t.document, Document::Docs(_)) {
                            // The editor already reconciled this user edit in its existing baseline.
                            t.pending_docs_edits = Some(Vec::new());
                        }
                    } else if let (Document::Sheets(old), Document::Sheets(new)) =
                        (&mut t.document, d)
                    {
                        old.selection = new.selection;
                        old.active = new.active;
                    }
                } else {
                    self.home(ui);
                }
            });
        if let Some(tail) = self.palette_tail.take() {
            self.open_palette(&ctx);
            ctx.input_mut(|i| i.events = tail);
        }
        if let Some((id, change)) = reject_ai {
            if let Err(e) = self.flush_pending(id) {
                self.error = e;
            } else if let Some(t) = self.tabs.iter_mut().find(|t| t.id == id)
                && let Err(e) = t.reject_ai(change)
            {
                self.error = e;
            }
        }
        for (id, action) in std::mem::take(&mut self.deferred) {
            match action {
                "activate_external" => external_activation = Some(id),
                "save" => self.save(id, false, false, false, None),
                "close" => self.close(id),
                "unlock" => self.request_unlock(id),
                "activate" | "home" => {
                    if let Some(old) = self.active
                        && let Err(e) = self.flush_pending(old)
                    {
                        self.error = e;
                        continue;
                    }
                    if action == "home" {
                        self.active = None;
                        user_activation = true;
                    } else if self.tabs.iter().any(|t| t.id == id) {
                        self.active = Some(id);
                        user_activation = true;
                    }
                }
                _ => {
                    if let Err(e) = self.flush_pending(id) {
                        self.error = e;
                        continue;
                    }
                    if let Some(t) = self.tabs.iter_mut().find(|t| t.id == id)
                        && let Err(e) = t.apply(if action == "undo" { Op::Undo } else { Op::Redo })
                    {
                        self.error = e;
                    }
                }
            }
        }
        match command {
            Some(c @ ("docs" | "sheets" | "slides")) => {
                let before = self.active;
                self.create(c);
                user_activation |= self.active != before;
            }
            Some("open") => self.send(Job::PickOpen),
            Some(action @ ("undo" | "redo")) => {
                if let Some(id) = self.active
                    && let Err(e) = self.flush_pending(id)
                {
                    self.error = e;
                } else if let Some(t) = self.tabs.iter_mut().find(|t| Some(t.id) == self.active)
                    && let Err(e) = t.apply(if action == "undo" { Op::Undo } else { Op::Redo })
                {
                    self.error = e;
                }
            }
            Some("save" | "saveas" | "copy" | "pdf") => {
                if let Some(id) = self.active {
                    self.save(
                        id,
                        command == Some("copy"),
                        command == Some("pdf"),
                        command == Some("saveas"),
                        if command == Some("pdf") {
                            Some("pdf")
                        } else {
                            None
                        },
                    );
                }
            }
            Some(
                ext @ ("docx" | "xlsx" | "pptx" | "txt" | "md" | "html" | "csv" | "tsv" | "png"),
            ) => {
                if let Some(id) = self.active {
                    self.save(
                        id,
                        false,
                        matches!(ext, "txt" | "md" | "html" | "csv" | "tsv" | "png"),
                        true,
                        Some(ext),
                    );
                }
            }
            Some("print") => {
                if let Some(id) = self.active {
                    self.write(
                        id,
                        crate::util::app_dir()
                            .join("print")
                            .join(format!("{id}.pdf")),
                        true,
                        true,
                        true,
                    );
                }
            }
            Some("close") => {
                if let Some(id) = self.active {
                    self.close(id);
                }
            }
            _ => {}
        }
        self.finish_mcp_closes();
        self.events();
        for (id, action) in std::mem::take(&mut self.deferred) {
            if action == "activate_external" {
                external_activation = Some(id);
            } else {
                self.deferred.push((id, action));
            }
        }
        if !user_activation
            && let Some(id) = external_activation
            && self.tabs.iter().any(|tab| tab.id == id)
        {
            if let Some(old) = self.active
                && let Err(error) = self.flush_pending(old)
            {
                self.error = error;
            } else {
                self.active = Some(id);
            }
        }
        let close_busy = self.opening
            || self.pending.is_some()
            || !self.picking.is_empty()
            || !self.saving.is_empty()
            || !self.reloading.is_empty();
        if ctx.input(|i| i.viewport().close_requested()) && close_busy && !self.allow_exit {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.status = "Завершите диалог или файловую операцию перед закрытием".into();
        }
        if ctx.input(|i| i.viewport().close_requested()) && !close_busy {
            let ids: Vec<_> = self.tabs.iter().map(|t| t.id).collect();
            for id in ids {
                if let Err(e) = self.flush_pending(id) {
                    self.error = e;
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                }
            }
        }
        if ctx.input(|i| i.viewport().close_requested())
            && !close_busy
            && !self.allow_exit
            && self.tabs.iter().any(Tab::dirty)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Pending::Exit);
        }
        if self.active != previous_active {
            crate::slides::editor::stop_presentation(&ctx);
        }
        self.dialogs(&ctx);
    }
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        let _ = self.settings.save();
    }
}

/// "New file" card of the start screen: a miniature of an empty file and a caption.
fn new_card(
    ui: &mut egui::Ui,
    kind: &str,
    label: &str,
    color: egui::Color32,
    width: f32,
) -> egui::Response {
    let preview_height = width * 0.62;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(width, preview_height + 34.0),
        egui::Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    let painter = ui.painter();
    let preview = egui::Rect::from_min_size(rect.min, egui::vec2(width, preview_height));
    let hovered = response.hovered();
    let line = crate::chrome::border(ui);
    painter.rect_filled(preview, 10.0, crate::chrome::workspace(ui));
    let page = preview.shrink2(egui::vec2(width * 0.16, 12.0));
    let page = if kind == "slides" {
        egui::Rect::from_center_size(
            preview.center(),
            egui::vec2(page.width(), (page.width() * 0.5625).min(page.height())),
        )
    } else {
        page
    };
    painter.rect_filled(page, 6.0, egui::Color32::WHITE);
    painter.rect_stroke(
        page,
        6.0,
        egui::Stroke::new(1.0, line),
        egui::StrokeKind::Inside,
    );
    let ink = egui::Color32::from_gray(214);
    match kind {
        "sheets" => {
            let cols = 4;
            let rows = 5;
            for c in 1..cols {
                let x = page.left() + page.width() * c as f32 / cols as f32;
                painter.vline(
                    x,
                    page.top() + 6.0..=page.bottom() - 6.0,
                    egui::Stroke::new(1.0, ink),
                );
            }
            for r in 1..rows {
                let y = page.top() + page.height() * r as f32 / rows as f32;
                painter.hline(
                    page.left() + 6.0..=page.right() - 6.0,
                    y,
                    egui::Stroke::new(1.0, ink),
                );
            }
            let head = egui::Rect::from_min_size(
                page.min + egui::vec2(6.0, 6.0),
                egui::vec2(page.width() - 12.0, page.height() / rows as f32 - 6.0),
            );
            painter.rect_filled(head, 2.0, color.gamma_multiply(0.18));
        }
        "slides" => {
            let title = egui::Rect::from_center_size(
                page.center() - egui::vec2(0.0, page.height() * 0.1),
                egui::vec2(page.width() * 0.6, 6.0),
            );
            painter.rect_filled(title, 3.0, color.gamma_multiply(0.55));
            let sub = egui::Rect::from_center_size(
                page.center() + egui::vec2(0.0, page.height() * 0.12),
                egui::vec2(page.width() * 0.38, 4.0),
            );
            painter.rect_filled(sub, 2.0, ink);
        }
        _ => {
            let mut y = page.top() + 12.0;
            let mut index = 0;
            while y < page.bottom() - 10.0 {
                let w = if index == 0 {
                    0.45
                } else if index % 4 == 3 {
                    0.6
                } else {
                    0.86
                };
                let bar = egui::Rect::from_min_size(
                    egui::pos2(page.left() + 10.0, y),
                    egui::vec2(
                        (page.width() - 20.0) * w,
                        if index == 0 { 5.0 } else { 3.0 },
                    ),
                );
                painter.rect_filled(
                    bar,
                    1.5,
                    if index == 0 {
                        color.gamma_multiply(0.55)
                    } else {
                        ink
                    },
                );
                y += if index == 0 { 12.0 } else { 8.0 };
                index += 1;
            }
        }
    }
    let edge = if hovered {
        egui::Stroke::new(2.0, color)
    } else {
        egui::Stroke::new(1.0, line)
    };
    painter.rect_stroke(preview, 10.0, edge, egui::StrokeKind::Inside);
    painter.text(
        egui::pos2(rect.left() + 2.0, preview.bottom() + 10.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::proportional(13.5),
        ui.visuals().text_color(),
    );
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}
/// Coloured letter badge of a recent file, by extension.
fn file_badge(path: &std::path::Path) -> (&'static str, egui::Color32) {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "xlsx" | "xlsm" | "xltx" | "xls" | "ods" | "csv" | "tsv" | "trinesheet" => {
            ("S", egui::Color32::from_rgb(33, 115, 70))
        }
        "pptx" | "pptm" | "potx" | "ppsx" | "odp" | "trineslide" => {
            ("P", egui::Color32::from_rgb(196, 62, 28))
        }
        _ => ("D", egui::Color32::from_rgb(43, 87, 154)),
    }
}
/// Document tab of the shell: a rounded pill with a coloured kind marker.
fn doc_tab(ui: &mut egui::Ui, label: &str, selected: bool, color: egui::Color32) -> egui::Response {
    let text = egui::RichText::new(label).size(12.5);
    let response = ui.add(
        egui::Button::new(if selected { text.strong() } else { text })
            .selected(selected)
            .frame_when_inactive(false)
            .corner_radius(6)
            .min_size(egui::vec2(0.0, 26.0)),
    );
    if color != egui::Color32::TRANSPARENT {
        let r = response.rect;
        ui.painter().rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(r.left() + 6.0, r.bottom() - 3.0),
                egui::vec2(r.width() - 12.0, 2.0),
            ),
            1.0,
            if selected {
                color
            } else {
                egui::Color32::TRANSPARENT
            },
        );
    }
    response
}

#[cfg(test)]
mod reload_tests {
    use super::*;
    fn fixture() -> Result<(Vec<Tab>, ReloadTarget), String> {
        let path = PathBuf::from("book.docx");
        let tab = Tab::new(
            10,
            Document::create("docs")?,
            "book".into(),
            Some(path.clone()),
        );
        Ok((
            vec![tab],
            ReloadTarget {
                tab_id: 10,
                base_version: 0,
                path,
            },
        ))
    }
    fn edit(tab: &mut Tab, author: Author) -> Result<(), String> {
        let mut document = tab.document.clone();
        if let Document::Docs(d) = &mut document {
            let id = d.paragraphs.first().ok_or("Нет абзаца")?.id;
            d.insert_text(id, 0, "new edit")?;
        }
        tab.apply(Op::Replace {
            document,
            label: "new edit".into(),
            author,
            base_version: None,
        })
    }
    #[test]
    fn ordinary_open_preserves_changed_tab_and_only_activates_it() -> Result<(), String> {
        let (mut tabs, target) = fixture()?;
        edit(
            &mut tabs[0],
            Author::Agent {
                name: "test".into(),
            },
        )?;
        let before = tabs[0].clone();
        let mut next_id = 11;
        // The watch notification does not turn an ordinary Open into a Reload.
        let disk_changed = Some(10);
        let prepared = prepare_open(
            &tabs,
            next_id,
            None,
            &target.path,
            Document::create("docs")?,
        )?;
        assert!(matches!(prepared, PreparedOpen::Activate(10)));
        assert_eq!(commit_open(&mut tabs, &mut next_id, prepared), 10);
        assert_eq!(disk_changed, Some(10));
        assert_eq!(next_id, 11);
        assert_eq!(tabs[0].document, before.document);
        assert_eq!(tabs[0].version, before.version);
        assert_eq!(tabs[0].undo.len(), before.undo.len());
        assert_eq!(tabs[0].ai.len(), before.ai.len());
        Ok(())
    }
    #[test]
    fn reload_completion_rejects_later_user_and_agent_edits() -> Result<(), String> {
        for author in [
            Author::User,
            Author::Agent {
                name: "test".into(),
            },
        ] {
            let (mut tabs, target) = fixture()?;
            edit(&mut tabs[0], author)?;
            let before = tabs[0].clone();
            assert!(
                prepare_open(
                    &tabs,
                    11,
                    Some(&target),
                    &target.path,
                    Document::create("docs")?
                )
                .is_err()
            );
            assert_eq!(tabs[0].document, before.document);
            assert_eq!(tabs[0].version, before.version);
            assert_eq!(tabs[0].undo.len(), before.undo.len());
            assert_eq!(tabs[0].ai.len(), before.ai.len());
        }
        Ok(())
    }
    #[test]
    fn successful_reload_has_new_namespace_and_clean_history() -> Result<(), String> {
        let (mut tabs, mut target) = fixture()?;
        edit(
            &mut tabs[0],
            Author::Agent {
                name: "test".into(),
            },
        )?;
        target.base_version = tabs[0].version;
        tabs.insert(
            0,
            Tab::new(9, Document::create("sheets")?, "left".into(), None),
        );
        tabs.push(Tab::new(
            11,
            Document::create("slides")?,
            "right".into(),
            None,
        ));
        let mut next_id = 12;
        let prepared = prepare_open(
            &tabs,
            next_id,
            Some(&target),
            &target.path,
            Document::create("docs")?,
        )?;
        assert_eq!(commit_open(&mut tabs, &mut next_id, prepared), 12);
        assert_eq!(next_id, 13);
        assert_eq!(
            tabs.iter().map(|t| t.id).collect::<Vec<_>>(),
            vec![9, 12, 11]
        );
        assert!(tabs[1].undo.is_empty());
        assert!(tabs[1].redo.is_empty());
        assert!(tabs[1].ai.is_empty());
        assert!(!tabs[1].dirty());
        let stale = crate::mcp::tools::dispatch(
            &mut tabs,
            &mut next_id,
            "get_document_info",
            &json!({"document_id":10}),
            "test",
        );
        assert!(stale.is_err_and(|e| e.starts_with("not_found")));
        let fresh = crate::mcp::tools::dispatch(
            &mut tabs,
            &mut next_id,
            "get_document_info",
            &json!({"document_id":12}),
            "test",
        )?;
        assert_eq!(fresh["id"], 12);
        Ok(())
    }
    #[test]
    fn reload_checks_closed_document_changed_path_and_counter_before_commit() -> Result<(), String>
    {
        let (tabs, target) = fixture()?;
        for next_id in [0, 10, u64::MAX] {
            assert!(
                prepare_open(
                    &tabs,
                    next_id,
                    Some(&target),
                    &target.path,
                    Document::create("docs")?
                )
                .is_err()
            );
        }
        let mut other_path = tabs.clone();
        other_path[0].path = Some(PathBuf::from("other.docx"));
        assert!(
            prepare_open(
                &other_path,
                11,
                Some(&target),
                &target.path,
                Document::create("docs")?
            )
            .is_err()
        );
        assert!(
            prepare_open(
                &tabs,
                11,
                Some(&target),
                std::path::Path::new("other.docx"),
                Document::create("docs")?
            )
            .is_err()
        );
        assert!(
            prepare_open(
                &[],
                11,
                Some(&target),
                &target.path,
                Document::create("docs")?
            )
            .is_err()
        );
        assert_eq!(tabs[0].id, 10);
        assert_eq!(tabs[0].version, 0);
        assert_eq!(tabs[0].path.as_ref(), Some(&target.path));
        Ok(())
    }
}

#[cfg(test)]
mod shell_input_tests {
    use super::*;
    use eframe::App as _;
    #[test]
    fn disabled_mcp_rejects_unknown_clients_without_approval_prompt() -> Result<(), String> {
        let mut app = fixture()?;
        let mut response = mcp_request(&mut app, "list_documents", json!({}))?;
        app.settings.mcp = false;
        app.allowed.clear();
        let ctx = app.editor_ctx.clone();
        app.mcp_requests(&ctx);
        assert!(
            response
                .try_recv()
                .map_err(|e| e.to_string())?
                .is_err_and(|e| e.starts_with("permission_denied:"))
        );
        assert!(app.clients.is_empty());
        Ok(())
    }
    #[test]
    fn rotated_slides_pdf_export_requires_confirmation() -> Result<(), String> {
        let mut app = fixture()?;
        let mut deck = crate::slides::model::Deck::default();
        let slide = deck.slides.first().ok_or("slide")?.id;
        let element = deck.add_element(slide, crate::slides::model::ElementKind::Rect)?;
        deck.slide_mut(slide)
            .and_then(|s| s.elements.iter_mut().find(|e| e.id == element))
            .ok_or("element")?
            .rotation = 45.0;
        app.tabs[0].document = Document::Slides(deck);
        worker_event(
            &mut app,
            Event::PickedSave {
                id: 10,
                path: std::env::temp_dir().join("rotated-slides.pdf"),
                copy: false,
                export: true,
            },
        )?;
        app.events();
        assert!(matches!(
            app.pending,
            Some(Pending::SaveFormat { id: 10, .. })
        ));
        assert!(!app.saving.contains(&10));
        Ok(())
    }
    #[test]
    fn palette_enter_runs_filtered_command_without_editing_document() -> Result<(), String> {
        let mut app = fixture()?;
        focus_page(&mut app)?;
        let before = text(&app);
        app.palette = true;
        app.palette_focus = true;
        app.palette_query = "  НАСТРОЙКИ  ".into();
        frame(&mut app, vec![]);
        frame(&mut app, vec![]);
        frame(
            &mut app,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(app.show_settings);
        assert!(!app.palette);
        assert_eq!(text(&app), before);
        assert!(palette_commands(false, "сохранить").is_empty());
        assert!(palette_commands(true, "несуществующая команда").is_empty());
        Ok(())
    }

    #[test]
    fn home_long_paths_leave_file_actions_within_viewport() -> Result<(), String> {
        for width in [480.0, 780.0, 1250.0] {
            let mut app = fixture()?;
            app.active = None;
            app.settings.recent = vec![PathBuf::from(format!(
                "C:/{}{}.docx",
                "long-directory/".repeat(30),
                "name".repeat(60)
            ))];
            let ctx = app.editor_ctx.clone();
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 820.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    app.home(ui);
                    assert!(
                        ui.min_rect().right() <= width + 1.0,
                        "home overflow at {width}: {:?}",
                        ui.min_rect()
                    );
                },
            );
            output.textures_delta.clear();
        }
        Ok(())
    }
    fn fixture() -> Result<App, String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let smoke = None;
        let settings = Settings {
            mcp: false,
            theme: "light".into(),
            ..Default::default()
        };
        let (mcp, bridge) = (None, None);
        let next_id = 12;
        let font_error = String::new();
        let mut app = App {
            shell_rects: BTreeMap::new(),
            editor_ctx: ctx.clone(),
            deferred: Vec::new(),
            smoke,
            tabs: Vec::new(),
            active: None,
            next_id,
            settings,
            worker: Worker::start(ctx.clone()),
            recovery: crate::recovery::Recovery::start_for_test(std::env::temp_dir().join(
                format!(
                    "TrineDocs-shell-{}",
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .map(|v| v.as_nanos())
                        .unwrap_or_default()
                ),
            )),
            locks: BTreeMap::new(),
            stamps: BTreeMap::new(),
            recoveries: Vec::new(),
            mcp,
            bridge,
            allowed: BTreeSet::new(),
            clients: BTreeSet::new(),
            mcp_saves: BTreeMap::new(),
            pending_mcp_closes: Vec::new(),
            pending: None,
            close_after_save: None,
            status: "Готово".into(),
            error: font_error,
            show_settings: false,
            show_ai: false,
            ai_tab: 0,
            chat: crate::chat::Chat::new(),
            palette: false,
            query: String::new(),
            palette_query: String::new(),
            palette_focus: false,
            palette_tail: None,
            last_snapshot: Instant::now(),
            last_watch: Instant::now(),
            snapshot_versions: BTreeMap::new(),
            disk_changed: None,
            last_agent: None,
            allow_exit: false,
            exit_after_save: false,
            saving: BTreeSet::new(),
            picking: BTreeSet::new(),
            opening: false,
            reloading: BTreeMap::new(),
        };
        app.tabs = vec![
            Tab::new(
                10,
                Document::Docs(crate::docs::model::Doc::from_text("original")),
                "First".into(),
                None,
            ),
            Tab::new(11, Document::create("sheets")?, "Second".into(), None),
        ];
        app.active = Some(10);
        Ok(app)
    }
    fn frame(app: &mut App, events: Vec<egui::Event>) {
        let ctx = app.editor_ctx.clone();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1250.0, 820.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.ui(ui, &mut eframe::Frame::_new_kittest()));
        output.textures_delta.clear();
    }
    fn pointer(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }
    fn focus_page(app: &mut App) -> Result<(), String> {
        frame(app, vec![]);
        frame(app, vec![]);
        let (rect, id) = app
            .editor_ctx
            .data(|data| data.get_temp::<(egui::Rect, egui::Id)>(egui::Id::new("docs-test-canvas")))
            .ok_or("canvas geometry")?;
        assert!(rect.is_positive(), "canvas must be visible: {rect:?}");
        let pos = rect.center();
        frame(
            app,
            vec![egui::Event::PointerMoved(pos), pointer(pos, true)],
        );
        frame(app, vec![pointer(pos, false)]);
        assert_eq!(
            app.editor_ctx.memory(|memory| memory.focused()),
            Some(id),
            "canvas focus"
        );
        Ok(())
    }
    fn text(app: &App) -> String {
        app.tabs
            .iter()
            .find(|t| t.id == 10)
            .and_then(|t| {
                if let Document::Docs(d) = &t.document {
                    Some(d.plain_text())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }
    fn frame_with_logic(app: &mut App, events: Vec<egui::Event>) {
        let ctx = app.editor_ctx.clone();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1250.0, 820.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            let mut frame = eframe::Frame::_new_kittest();
            app.logic(ui.ctx(), &mut frame);
            app.ui(ui, &mut frame);
        });
        output.textures_delta.clear();
    }
    type McpReply = tokio::sync::oneshot::Receiver<Result<Value, String>>;
    fn mcp_request(app: &mut App, tool: &str, args: Value) -> Result<McpReply, String> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let (reply, response) = tokio::sync::oneshot::channel();
        let client = "test-agent".to_string();
        app.settings.mcp = true;
        app.allowed.insert(client.clone());
        app.bridge = Some(receiver);
        sender
            .send(crate::mcp::BridgeRequest {
                tool: tool.into(),
                args,
                client,
                reply,
                deadline: Instant::now() + std::time::Duration::from_secs(10),
            })
            .map_err(|error| error.to_string())?;
        Ok(response)
    }
    fn worker_event(app: &mut App, event: Event) -> Result<(), String> {
        let (sender, receiver) = std::sync::mpsc::channel();
        app.worker.rx = receiver;
        sender.send(event).map_err(|error| error.to_string())
    }
    #[test]
    fn mcp_dry_run_and_unknown_tools_preserve_pending_sheet_input() -> Result<(), String> {
        for tool in ["sheets_write_range", "unknown_tool"] {
            let mut app = fixture()?;
            app.active = Some(11);
            frame(&mut app, vec![]);
            let canvas_id = egui::Id::new(("sheets-editor", 11_u64)).with("canvas");
            app.editor_ctx
                .memory_mut(|memory| memory.request_focus(canvas_id));
            frame(&mut app, vec![egui::Event::Text("draft".into())]);
            let tab = app
                .tabs
                .iter()
                .find(|tab| tab.id == 11)
                .ok_or("sheet tab")?;
            let before = tab.document.clone();
            let version = tab.version;
            let undo = tab.undo.len();
            let sheet_id = match &before {
                Document::Sheets(book) => book.active_id(),
                _ => return Err("sheet kind".into()),
            };
            let mut response = mcp_request(
                &mut app,
                tool,
                json!({
                    "document_id":11, "sheet_id":sheet_id, "row":0, "col":1,
                    "values":[["agent"]],
                    "mode":if tool == "sheets_write_range" { "dry_run" } else { "apply" }
                }),
            )?;
            let ctx = app.editor_ctx.clone();
            app.mcp_requests(&ctx);
            let result = response.try_recv().map_err(|error| error.to_string())?;
            if tool == "sheets_write_range" {
                assert_eq!(result?["applied"], false);
            } else {
                assert!(result.is_err_and(|error| error.starts_with("unsupported_feature")));
            }
            let tab = app
                .tabs
                .iter()
                .find(|tab| tab.id == 11)
                .ok_or("sheet tab")?;
            assert_eq!(tab.document, before);
            assert_eq!(tab.version, version);
            assert_eq!(tab.undo.len(), undo);
            app.flush_pending(11)?;
            let tab = app
                .tabs
                .iter()
                .find(|tab| tab.id == 11)
                .ok_or("sheet tab")?;
            let Document::Sheets(book) = &tab.document else {
                return Err("sheet kind".into());
            };
            assert_eq!(
                book.active_sheet()
                    .and_then(|sheet| sheet.cell(0, 0))
                    .map(|cell| cell.input.as_str()),
                Some("draft")
            );
            assert_eq!(tab.version, version + 1);
        }
        Ok(())
    }
    #[test]
    fn queued_mcp_close_rechecks_revoked_permission() -> Result<(), String> {
        for disable_mcp in [false, true] {
            let mut app = fixture()?;
            let mut response = mcp_request(&mut app, "close_document", json!({"document_id":10}))?;
            let ctx = app.editor_ctx.clone();
            app.mcp_requests(&ctx);
            assert_eq!(app.pending_mcp_closes.len(), 1);
            if disable_mcp {
                app.settings.mcp = false;
            } else {
                app.allowed.clear();
            }
            app.finish_mcp_closes();
            assert!(
                response
                    .try_recv()
                    .map_err(|error| error.to_string())?
                    .is_err_and(|error| error.starts_with("permission_denied"))
            );
            assert!(app.tabs.iter().any(|tab| tab.id == 10));
            assert!(app.pending.is_none());
        }
        Ok(())
    }
    fn second_doc(app: &mut App) -> Result<(), String> {
        let tab = app
            .tabs
            .iter_mut()
            .find(|tab| tab.id == 11)
            .ok_or("second tab")?;
        tab.document = Document::Docs(crate::docs::model::Doc::from_text("second"));
        Ok(())
    }
    fn second_paragraph(app: &App) -> Result<u64, String> {
        match app
            .tabs
            .iter()
            .find(|tab| tab.id == 11)
            .map(|tab| &tab.document)
        {
            Some(Document::Docs(doc)) => doc
                .paragraphs
                .first()
                .map(|p| p.id)
                .ok_or_else(|| "paragraph".to_string()),
            _ => Err("second doc".into()),
        }
    }
    #[test]
    fn mcp_edit_in_another_document_preserves_active_document_input() -> Result<(), String> {
        let mut app = fixture()?;
        second_doc(&mut app)?;
        focus_page(&mut app)?;
        let paragraph = second_paragraph(&app)?;
        let mut response = mcp_request(
            &mut app,
            "docs_insert_text",
            json!({
                "document_id":11, "paragraph_id":paragraph, "offset":0, "text":"agent "
            }),
        )?;
        frame_with_logic(&mut app, vec![egui::Event::Text("local".into())]);
        let result = response.try_recv().map_err(|error| error.to_string())??;
        assert_eq!(result["applied"], true);
        assert_eq!(app.active, Some(10));
        assert!(text(&app).contains("local"));
        let target = app.tabs.iter().find(|tab| tab.id == 11).ok_or("target")?;
        let Document::Docs(doc) = &target.document else {
            return Err("target kind".into());
        };
        assert_eq!(doc.plain_text(), "agent second");
        assert_eq!(target.ai.len(), 1);
        assert!(
            app.last_agent
                .as_ref()
                .is_some_and(|(id, name, _)| *id == 11 && name == "test-agent")
        );
        Ok(())
    }
    #[test]
    fn mcp_creation_activates_after_current_document_input() -> Result<(), String> {
        let mut app = fixture()?;
        focus_page(&mut app)?;
        let mut response = mcp_request(
            &mut app,
            "create_document",
            json!({
                "kind":"docs", "title":"Agent document"
            }),
        )?;
        frame_with_logic(&mut app, vec![egui::Event::Text("prefix".into())]);
        let result = response.try_recv().map_err(|error| error.to_string())??;
        assert_eq!(result["applied"], true);
        assert_eq!(app.active, result["document_id"].as_u64());
        assert_ne!(app.active, Some(10));
        assert!(text(&app).contains("prefix"));
        let target = app.current().ok_or("created tab")?;
        let Document::Docs(doc) = &target.document else {
            return Err("created kind".into());
        };
        assert!(
            doc.plain_text().is_empty(),
            "old document input must not enter the created document"
        );
        Ok(())
    }
    #[test]
    fn busy_mcp_reconciliation_activates_after_current_document_input() -> Result<(), String> {
        let mut app = fixture()?;
        second_doc(&mut app)?;
        focus_page(&mut app)?;
        app.tabs
            .iter_mut()
            .find(|tab| tab.id == 11)
            .ok_or("second tab")?
            .pending_docs_edits = None;
        let paragraph = second_paragraph(&app)?;
        let mut response = mcp_request(
            &mut app,
            "docs_insert_text",
            json!({
                "document_id":11, "paragraph_id":paragraph, "offset":0, "text":"agent "
            }),
        )?;
        frame_with_logic(&mut app, vec![egui::Event::Text("prefix".into())]);
        let result = response.try_recv().map_err(|error| error.to_string())?;
        assert!(result.is_err_and(|error| error.starts_with("conflict: дождитесь отображения")));
        assert!(text(&app).contains("prefix"));
        assert_eq!(app.active, Some(11));
        let target = app.current().ok_or("target")?;
        let Document::Docs(doc) = &target.document else {
            return Err("target kind".into());
        };
        assert_eq!(doc.plain_text(), "second");
        Ok(())
    }
    #[test]
    fn asynchronous_open_activates_after_current_document_input() -> Result<(), String> {
        let mut app = fixture()?;
        focus_page(&mut app)?;
        worker_event(
            &mut app,
            Event::Opened {
                reload: None,
                path: PathBuf::from("external.docx"),
                document: Document::Docs(crate::docs::model::Doc::from_text("loaded")),
                lock: None,
                stamp: None,
            },
        )?;
        frame(&mut app, vec![egui::Event::Text("prefix".into())]);
        assert!(text(&app).contains("prefix"));
        assert_eq!(app.active, Some(12));
        let target = app.current().ok_or("opened tab")?;
        let Document::Docs(doc) = &target.document else {
            return Err("opened kind".into());
        };
        assert_eq!(doc.plain_text(), "loaded");
        Ok(())
    }
    #[test]
    fn asynchronous_reload_rejects_current_frame_edits() -> Result<(), String> {
        let mut app = fixture()?;
        focus_page(&mut app)?;
        let path = PathBuf::from("current.docx");
        let tab = app.tabs.iter_mut().find(|tab| tab.id == 10).ok_or("tab")?;
        tab.path = Some(path.clone());
        let target = ReloadTarget {
            tab_id: 10,
            base_version: tab.version,
            path: path.clone(),
        };
        app.reloading.insert(10, target.clone());
        worker_event(
            &mut app,
            Event::Opened {
                reload: Some(target),
                path,
                document: Document::Docs(crate::docs::model::Doc::from_text("disk contents")),
                lock: None,
                stamp: None,
            },
        )?;
        frame(&mut app, vec![egui::Event::Text("prefix".into())]);
        assert!(text(&app).contains("prefix"));
        assert!(!text(&app).contains("disk contents"));
        assert_eq!(app.active, Some(10));
        assert!(app.error.starts_with("conflict:"));
        assert_eq!(app.disk_changed, Some(10));
        Ok(())
    }
    #[test]
    fn save_completion_does_not_close_over_current_frame_edits() -> Result<(), String> {
        let mut app = fixture()?;
        focus_page(&mut app)?;
        let tab = app.tabs.iter_mut().find(|tab| tab.id == 10).ok_or("tab")?;
        tab.saved_version = tab.version;
        let version = tab.version;
        app.close_after_save = Some(10);
        app.saving.insert(10);
        worker_event(
            &mut app,
            Event::Saved {
                id: 10,
                version,
                path: PathBuf::from("current.docx"),
                copy: false,
                export: false,
                stamp: None,
                lock: None,
            },
        )?;
        frame(&mut app, vec![egui::Event::Text("prefix".into())]);
        assert!(text(&app).contains("prefix"));
        assert!(app.tabs.iter().any(|tab| tab.id == 10 && tab.dirty()));
        assert!(matches!(app.pending, Some(Pending::Close(10))));
        Ok(())
    }
    #[test]
    fn mcp_close_preserves_current_frame_input_before_confirmation() -> Result<(), String> {
        let mut app = fixture()?;
        focus_page(&mut app)?;
        let tab = app.tabs.iter_mut().find(|tab| tab.id == 10).ok_or("tab")?;
        tab.saved_version = tab.version;
        let mut response = mcp_request(&mut app, "close_document", json!({"document_id":10}))?;
        frame_with_logic(&mut app, vec![egui::Event::Text("prefix".into())]);
        let result = response.try_recv().map_err(|error| error.to_string())??;
        assert_eq!(result["applied"], false);
        assert_eq!(result["needs_confirmation"], true);
        assert!(text(&app).contains("prefix"));
        assert!(app.tabs.iter().any(|tab| tab.id == 10 && tab.dirty()));
        assert!(matches!(app.pending, Some(Pending::Close(10))));
        Ok(())
    }
    #[test]
    fn mcp_close_rechecks_base_version_after_current_frame_input() -> Result<(), String> {
        let mut app = fixture()?;
        focus_page(&mut app)?;
        let tab = app.tabs.iter_mut().find(|tab| tab.id == 10).ok_or("tab")?;
        tab.saved_version = tab.version;
        let version = tab.version;
        let mut response = mcp_request(
            &mut app,
            "close_document",
            json!({
                "document_id":10, "base_version":version
            }),
        )?;
        frame_with_logic(&mut app, vec![egui::Event::Text("prefix".into())]);
        let result = response.try_recv().map_err(|error| error.to_string())?;
        assert!(result.is_err_and(|error| error.starts_with("conflict:")));
        assert!(text(&app).contains("prefix"));
        assert_eq!(app.active, Some(10));
        assert!(app.pending.is_none());
        Ok(())
    }
    #[test]
    fn palette_shortcut_routes_text_to_search_without_changing_document() -> Result<(), String> {
        let mut app = fixture()?;
        focus_page(&mut app)?;
        let before = text(&app);
        let cmd = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        frame(
            &mut app,
            vec![
                egui::Event::ModifiersChanged(cmd),
                egui::Event::Key {
                    key: egui::Key::K,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: cmd,
                },
                egui::Event::Text("needle".into()),
            ],
        );
        assert!(app.palette);
        assert_eq!(app.palette_query, "needle");
        assert_eq!(text(&app), before);
        assert!(app.query.is_empty());
        frame(&mut app, vec![egui::Event::Text("2".into())]);
        assert_eq!(app.palette_query, "needle2");
        assert_eq!(text(&app), before);
        Ok(())
    }
    #[test]
    fn palette_preserves_text_before_shortcut_and_routes_only_later_text() -> Result<(), String> {
        let mut app = fixture()?;
        focus_page(&mut app)?;
        let cmd = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        frame(
            &mut app,
            vec![
                egui::Event::Text("prefix".into()),
                egui::Event::Key {
                    key: egui::Key::K,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: cmd,
                },
                egui::Event::Text("needle".into()),
            ],
        );
        assert!(text(&app).contains("prefix"));
        assert!(!text(&app).contains("needle"));
        assert_eq!(app.palette_query, "needle");
        Ok(())
    }
    #[test]
    fn shell_actions_keep_last_frame_text_before_switch_or_close() -> Result<(), String> {
        for (id, action) in [(0, "home"), (11, "activate"), (10, "close")] {
            let mut app = fixture()?;
            focus_page(&mut app)?;
            frame(&mut app, vec![egui::Event::Text("ready".into())]);
            let before = text(&app);
            assert!(before.contains("ready"), "page must have focus");
            let t = app.tabs.iter_mut().find(|t| t.id == 10).ok_or("tab")?;
            t.saved_version = t.version;
            let pos = app
                .shell_rects
                .get(&(id, action))
                .ok_or("shell control")?
                .center();
            frame(
                &mut app,
                vec![egui::Event::PointerMoved(pos), pointer(pos, true)],
            );
            frame(
                &mut app,
                vec![egui::Event::Text("late".into()), pointer(pos, false)],
            );
            assert!(text(&app).contains("late"), "{action} lost final input");
            if action == "close" {
                assert!(matches!(app.pending, Some(Pending::Close(10))));
                let protected = text(&app);
                frame(
                    &mut app,
                    vec![
                        egui::Event::Text("blocked".into()),
                        egui::Event::Paste("blocked paste".into()),
                        egui::Event::Key {
                            key: egui::Key::Delete,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
                assert_eq!(
                    text(&app),
                    protected,
                    "confirmation must block editor events"
                );
            } else {
                assert_eq!(app.active, if id == 0 { None } else { Some(id) });
            }
        }
        Ok(())
    }
}
