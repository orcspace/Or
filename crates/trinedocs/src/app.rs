use crate::{
    file_io::{Event, FileLock, Job, Worker},
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
    SaveFormat {
        id: u64,
        path: PathBuf,
        copy: bool,
        export: bool,
    },
}
pub struct App {
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
    pending: Option<Pending>,
    close_after_save: Option<u64>,
    status: String,
    error: String,
    show_settings: bool,
    show_ai: bool,
    palette: bool,
    query: String,
    last_snapshot: Instant,
    last_watch: Instant,
    snapshot_versions: BTreeMap<u64, u64>,
    disk_changed: Option<u64>,
    last_agent: Option<(u64, String, Instant)>,
    allow_exit: bool,
    exit_after_save: bool,
    saving: BTreeSet<u64>,
}
impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let smoke = crate::ui_smoke::State::from_args();
        let settings = if smoke.is_some() {
            Settings {
                mcp: false,
                ..Default::default()
            }
        } else {
            Settings::load()
        };
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
            pending: None,
            close_after_save: None,
            status: "Готово".into(),
            error: font_error,
            show_settings: false,
            show_ai: false,
            palette: false,
            query: String::new(),
            last_snapshot: Instant::now(),
            last_watch: Instant::now(),
            snapshot_versions: BTreeMap::new(),
            disk_changed: None,
            last_agent: None,
            allow_exit: false,
            exit_after_save: false,
            saving: BTreeSet::new(),
        };
        for arg in std::env::args().skip(1).filter(|s| !s.starts_with('-')) {
            if app.smoke.is_none() {
                let _ = app.worker.send(Job::Open(PathBuf::from(arg)));
            }
        }
        app
    }
    fn tr<'a>(&self, s: &'a str) -> &'a str {
        crate::i18n::tr(s, &self.settings.language)
    }
    fn create(&mut self, kind: &str) {
        match Document::create(kind) {
            Ok(d) => {
                let id = self.next_id;
                self.next_id = self.next_id.saturating_add(1);
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
        if let Err(e) = self.worker.send(job) {
            self.error = e
        }
    }
    fn current(&self) -> Option<&Tab> {
        self.tabs.iter().find(|t| Some(t.id) == self.active)
    }
    fn save(&mut self, id: u64, copy: bool, export: bool, choose: bool, extension: Option<&str>) {
        if self.saving.contains(&id) {
            self.error = "Сохранение уже выполняется".into();
            return;
        }
        let Some(t) = self.tabs.iter().find(|t| t.id == id) else {
            return;
        };
        if choose || t.path.is_none() || copy || export {
            let ext = extension.unwrap_or(t.document.extension()).to_string();
            self.send(Job::PickSave {
                id,
                extension: ext,
                copy,
                export,
            });
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
    fn write(&mut self, id: u64, path: PathBuf, copy: bool, export: bool, print: bool) {
        let Some(t) = self.tabs.iter().find(|t| t.id == id) else {
            return;
        };
        let expected = if t.path.as_ref() == Some(&path) {
            self.stamps.get(&id).copied().flatten()
        } else {
            None
        };
        if t.read_only && t.path.as_ref() == Some(&path) && !export {
            self.error =
                "Исходный файл открыт только для чтения; сохраните под другим именем".into();
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
    fn remove(&mut self, id: u64) {
        if let Some(reply) = self.mcp_saves.remove(&id) {
            let _ = reply.send(Err("Документ закрыт".into()));
        }
        self.tabs.retain(|t| t.id != id);
        self.locks.remove(&id);
        self.stamps.remove(&id);
        self.snapshot_versions.remove(&id);
        self.recovery.clear(id);
        if self.active == Some(id) {
            self.active = self.tabs.last().map(|t| t.id);
        }
    }
    fn close(&mut self, id: u64) {
        if self.tabs.iter().any(|t| t.id == id && t.dirty()) {
            self.pending = Some(Pending::Close(id));
        } else {
            self.remove(id);
        }
    }
    fn events(&mut self) {
        while let Ok(event) = self.worker.rx.try_recv() {
            match event {
                Event::RegistrationPreview(p) => self.pending = Some(Pending::Register(p)),
                Event::Registered => {
                    self.status = "Клиент MCP зарегистрирован. Перезапустите клиент.".into()
                }
                Event::PickedOpen(path) => self.send(Job::Open(path)),
                Event::PickedSave {
                    id,
                    path,
                    copy,
                    export,
                } => {
                    if self.tabs.iter().find(|t| t.id == id).is_some_and(|t| {
                        crate::native::extension(&path) != t.document.extension()
                            && crate::native::extension(&path) != "pdf"
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
                    path,
                    document,
                    lock,
                    stamp,
                } => {
                    if let Some(t) = self
                        .tabs
                        .iter_mut()
                        .find(|t| t.path.as_ref() == Some(&path))
                    {
                        if self.disk_changed != Some(t.id) {
                            self.active = Some(t.id);
                            continue;
                        }
                        t.document = document;
                        t.version = t.version.saturating_add(1);
                        t.saved_version = t.version;
                        t.undo.clear();
                        t.redo.clear();
                        self.stamps.insert(t.id, stamp);
                        self.active = Some(t.id);
                        self.disk_changed = None;
                    } else {
                        let id = self.next_id;
                        self.next_id = self.next_id.saturating_add(1);
                        let title = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Без названия".into());
                        let mut t = Tab::new(id, document, title, Some(path.clone()));
                        t.read_only = lock.is_none();
                        if let Some(l) = lock {
                            self.locks.insert(id, l);
                        }
                        self.stamps.insert(id, stamp);
                        self.tabs.push(t);
                        self.active = Some(id);
                    }
                    self.settings.remember(path);
                    self.status = "Файл открыт".into();
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
                            t.read_only = false;
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
                        if self.stamps.get(&id).is_some_and(|old| *old != new) {
                            self.disk_changed = Some(id);
                        }
                    }
                }
            }
        }
    }
    fn mcp_requests(&mut self) {
        for _ in 0..16 {
            let Some(r) = self.bridge.as_ref().and_then(|b| b.try_recv().ok()) else {
                break;
            };
            if r.reply.is_closed() || Instant::now() > r.deadline {
                continue;
            }
            if !self.allowed.contains(&r.client) {
                self.clients.insert(r.client);
                let _ = r.reply.send(Err(
                    "needs_confirmation: разрешите клиент в окне TrineDocs".into()
                ));
                continue;
            }
            if !self.settings.mcp {
                let _ = r.reply.send(Err("permission_denied: MCP отключён".into()));
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
            let id = r.args.get("document_id").and_then(Value::as_u64);
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
            ) && r.args.get("mode").and_then(Value::as_str) == Some("dry_run")
            {
                let _ = r.reply.send(Ok(json!({"applied":false})));
                continue;
            }
            if r.tool == "save_document" {
                if let Some(id) = id
                    && self
                        .tabs
                        .iter()
                        .any(|t| t.id == id && t.path.is_some() && !t.read_only)
                    && !self.saving.contains(&id)
                    && !self.mcp_saves.contains_key(&id)
                {
                    self.mcp_saves.insert(id, r.reply);
                    self.save(id, false, false, false, None);
                } else {
                    let _ = r.reply.send(Err(
                        "Для первого сохранения выберите путь через интерфейс".into(),
                    ));
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
                if let Some(id) = id {
                    self.close(id);
                    let _ = r
                        .reply
                        .send(Ok(json!({"applied":!self.tabs.iter().any(|t|t.id==id)})));
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
            if result.is_ok() && !crate::mcp::tools::read_only(&r.tool) {
                self.active = id.or_else(|| self.tabs.last().map(|t| t.id));
                if let Some(id) = self.active {
                    self.last_agent = Some((id, r.client.clone(), Instant::now()));
                }
            }
            let _ = r.reply.send(result);
        }
    }
    fn tick(&mut self, ctx: &egui::Context) {
        let previous_active = self.active;
        self.mcp_saves.retain(|_, reply| !reply.is_closed());
        self.events();
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
        self.mcp_requests();
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
        if crate::slides::editor::is_presenting(ctx) {
            return;
        }
        let cmd = egui::Modifiers::COMMAND;
        if ctx.input_mut(|i| i.consume_key(cmd, egui::Key::O)) {
            self.send(Job::PickOpen);
        }
        if ctx.input_mut(|i| i.consume_key(cmd, egui::Key::S))
            && let Some(id) = self.active
        {
            self.save(id, false, false, false, None);
        }
        if ctx.input_mut(|i| i.consume_key(cmd, egui::Key::W))
            && let Some(id) = self.active
        {
            self.close(id);
        }
        if ctx.input_mut(|i| i.consume_key(cmd, egui::Key::K)) {
            self.palette = !self.palette;
        }
        if ctx.input_mut(|i| i.consume_key(cmd, egui::Key::Z))
            && let Some(t) = self.tabs.iter_mut().find(|t| Some(t.id) == self.active)
            && let Err(e) = t.apply(Op::Undo)
        {
            self.error = e
        }
        if ctx.input_mut(|i| i.consume_key(cmd, egui::Key::Y))
            && let Some(t) = self.tabs.iter_mut().find(|t| Some(t.id) == self.active)
            && let Err(e) = t.apply(Op::Redo)
        {
            self.error = e
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Tab))
            && !self.tabs.is_empty()
        {
            let index = self
                .tabs
                .iter()
                .position(|t| Some(t.id) == self.active)
                .unwrap_or(self.tabs.len() - 1);
            self.active = self.tabs.get((index + 1) % self.tabs.len()).map(|t| t.id);
        }
        for f in ctx.input(|i| i.raw.dropped_files.clone()) {
            self.send(Job::Open(f.path().to_path_buf()));
        }
    }
    fn home(&mut self, ui: &mut egui::Ui) {
        ui.add_space(32.0);
        ui.heading("TrineDocs");
        ui.label("Docs · Sheets · Slides");
        ui.add_space(24.0);
        ui.horizontal(|ui| {
            for (kind, label, c) in [
                ("docs", "Document", [62, 123, 224]),
                ("sheets", "Spreadsheet", [49, 153, 103]),
                ("slides", "Presentation", [225, 120, 56]),
            ] {
                if ui
                    .add_sized(
                        [210.0, 92.0],
                        egui::Button::new(egui::RichText::new(self.tr(label)).size(21.0))
                            .fill(egui::Color32::from_rgb(c[0], c[1], c[2])),
                    )
                    .clicked()
                {
                    self.create(kind);
                }
            }
        });
        ui.add_space(24.0);
        ui.heading(self.tr("Recent files"));
        ui.text_edit_singleline(&mut self.query);
        let recent = self.settings.recent.clone();
        let q = self.query.to_lowercase();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for p in recent
                .iter()
                .filter(|p| p.to_string_lossy().to_lowercase().contains(&q))
            {
                ui.horizontal(|ui| {
                    let pin = self.settings.pinned.contains(p);
                    if ui.button(if pin { "★" } else { "☆" }).clicked() {
                        if pin {
                            self.settings.pinned.retain(|v| v != p)
                        } else {
                            self.settings.pinned.push(p.clone())
                        }
                        if let Err(e) = self.settings.save() {
                            self.error = e;
                        }
                    }
                    if ui
                        .button(
                            p.file_name()
                                .map(|s| s.to_string_lossy())
                                .unwrap_or_default(),
                        )
                        .clicked()
                    {
                        self.send(Job::Open(p.clone()));
                    }
                    ui.label(egui::RichText::new(p.display().to_string()).weak().small());
                    if ui.small_button("×").clicked() {
                        self.settings.recent.retain(|v| v != p);
                        if let Err(e) = self.settings.save() {
                            self.error = e;
                        }
                    }
                });
                ui.separator();
            }
        });
        ui.add_space(20.0);
        ui.label(
            egui::RichText::new(
                "Предварительная сборка. Полный паритет с Microsoft Office ещё не достигнут.",
            )
            .weak(),
        );
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
                                let id = self.next_id;
                                self.next_id = self.next_id.saturating_add(1);
                                let mut t = Tab::new(id, s.document.clone(), s.title.clone(), None);
                                t.saved_version = u64::MAX;
                                self.tabs.push(t);
                                self.active = Some(id);
                                self.recovery.clear(s.id);
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
                if ui.button("Перезагрузить").clicked()&& let Some(p)=self.tabs.iter().find(|t|t.id==id).and_then(|t|t.path.clone()){self.send(Job::Open(p));}
                if ui.button("Оставить правки").clicked(){self.stamps.insert(id, self.tabs.iter().find(|t|t.id==id).and_then(|t|t.path.as_ref()).and_then(|p|crate::file_io::stamp(p)));self.disk_changed=None;}
            });
        });
        }
        if self.pending.is_some() {
            let mut action = 0;
            egui::Window::new("Подтверждение").collapsible(false).show(ctx,|ui|{
            match self.pending.as_ref(){Some(Pending::Register(p))=>{ui.label(format!("Добавить сервер trinedocs в {}",p.path.display()));ui.label("Остальные записи сохранятся. Существующая запись trinedocs не заменяется.");ui.monospace(&p.change);ui.horizontal(|ui|{if ui.button("Добавить").clicked(){action=1}
if ui.button("Отмена").clicked(){action=3}});},Some(Pending::SaveFormat{id,..})=>{if let Some(t)=self.tabs.iter().find(|t|t.id==*id){match &t.document{Document::Docs(d)=>{for loss in &d.losses{ui.colored_label(egui::Color32::YELLOW,loss);}},Document::Slides(d)=>{for loss in d.export_warnings(){ui.colored_label(egui::Color32::YELLOW,loss);}},_=>{}}}ui.label("Office/текстовый экспорт сохраняет поддерживаемые функции. Неподдерживаемые части могут быть потеряны; полная модель сохраняется в нативном формате.");ui.horizontal(|ui|{if ui.button("Продолжить").clicked(){action=1}
if ui.button("Отмена").clicked(){action=3}});}
            _=>{ui.label("Сохранить изменения?");ui.horizontal(|ui|{if ui.button("Сохранить").clicked(){action=1}
if ui.button("Не сохранять").clicked(){action=2}
if ui.button("Отмена").clicked(){action=3}});}}
        });
            if action != 0
                && let Some(p) = self.pending.take()
            {
                match (p.clone(), action) {
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
                            Pending::SaveFormat { id, .. } | Pending::Close(id) => Some(*id),
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
            egui::Window::new("Команды · Ctrl+K")
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.text_edit_singleline(&mut self.query);
                    let q = self.query.to_lowercase();
                    for (k, l) in [
                        ("docs", "Создать документ"),
                        ("sheets", "Создать таблицу"),
                        ("slides", "Создать презентацию"),
                    ] {
                        if l.to_lowercase().contains(&q) && ui.button(l).clicked() {
                            self.create(k);
                            self.palette = false;
                        }
                    }
                    if ui.button("Открыть файл").clicked() {
                        self.send(Job::PickOpen);
                        self.palette = false;
                    }
                    if ui.button("Настройки").clicked() {
                        self.show_settings = true;
                        self.palette = false;
                    }
                });
            self.palette &= open;
        }
    }
}
impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
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
        if ctx.input(|i| i.viewport().close_requested())
            && !self.allow_exit
            && self.tabs.iter().any(Tab::dirty)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Pending::Exit);
        }
        let mut command = None;
        egui::Panel::top("shell").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong("TrineDocs");
                ui.separator();
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
                                    Document::Docs(_) => &["trinedoc", "docx", "txt", "md", "html"],
                                    Document::Sheets(_) => {
                                        &["trinesheet", "xlsx", "csv", "tsv", "html"]
                                    }
                                    Document::Slides(_) => &["trineslide", "pptx", "png"],
                                };
                                for ext in formats {
                                    if ui
                                        .button(if *ext == "png" {
                                            "PNG · первый слайд".into()
                                        } else {
                                            ext.to_uppercase()
                                        })
                                        .clicked()
                                    {
                                        command = Some(*ext);
                                        ui.close();
                                    }
                                }
                            },
                        );
                    }
                });
                if ui.button(self.tr("Open")).clicked() {
                    command = Some("open")
                }
                if ui
                    .add_enabled(self.active.is_some(), egui::Button::new(self.tr("Save")))
                    .clicked()
                {
                    command = Some("save")
                }
                if ui.button(self.tr("Settings")).clicked() {
                    self.show_settings = true
                }
                if ui.button(self.tr("AI changes")).clicked() {
                    self.show_ai = !self.show_ai
                }
                if ui.button("Ctrl+K").clicked() {
                    self.palette = true
                }
            });
            ui.horizontal_wrapped(|ui| {
                if ui
                    .selectable_label(self.active.is_none(), self.tr("Home"))
                    .clicked()
                {
                    self.active = None;
                }
                let mut closed = None;
                for t in &self.tabs {
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(
                                self.active == Some(t.id),
                                format!("{}{}", t.title, if t.dirty() { " •" } else { "" }),
                            )
                            .clicked()
                        {
                            self.active = Some(t.id)
                        }
                        if ui.small_button("×").clicked() {
                            closed = Some(t.id)
                        }
                    });
                }
                if let Some(id) = closed {
                    self.close(id);
                }
            });
        });
        match command {
            Some(c @ ("docs" | "sheets" | "slides")) => self.create(c),
            Some("open") => self.send(Job::PickOpen),
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
                ext @ ("trinedoc" | "trinesheet" | "trineslide" | "docx" | "xlsx" | "pptx" | "txt"
                | "md" | "html" | "csv" | "tsv" | "png"),
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
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.small(&self.status);
                if let Some(t) = self.current() {
                    ui.separator();
                    ui.small(format!("{} · v{}", t.document.kind(), t.version));
                    if t.read_only {
                        ui.colored_label(egui::Color32::YELLOW, "Только чтение: lock-файл");
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
        if self.show_ai {
            egui::Panel::right("ai-review")
                .default_size(270.0)
                .show(ui, |ui| {
                    ui.heading("Изменения AI");
                    if let Some(t) = self.tabs.iter_mut().find(|t| Some(t.id) == self.active) {
                        let mut reject = None;
                        for c in t.ai.clone().iter().rev() {
                            if !c.accepted {
                                ui.label(&c.label);
                                ui.horizontal(|ui| {
                                    if ui.button("Принять").clicked()
                                        && let Some(v) = t.ai.iter_mut().find(|v| v.id == c.id)
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
                        if let Some(id) = reject
                            && let Err(e) = t.reject_ai(id)
                        {
                            self.error = e
                        }
                    }
                });
        }
        egui::CentralPanel::default().show(ui, |ui| {
            if let Some(t) = self.tabs.iter_mut().find(|t| Some(t.id) == self.active) {
                let mut d = t.document.clone();
                let changed = ui
                    .push_id(t.id, |ui| {
                        ui.add_enabled_ui(!t.read_only, |ui| match &mut d {
                            Document::Docs(d) => crate::docs::editor::show(ui, d),
                            Document::Sheets(b) => crate::sheets::editor::show(ui, b),
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
                    }
                } else if let (Document::Sheets(old), Document::Sheets(new)) = (&mut t.document, d)
                {
                    old.selection = new.selection;
                    old.active = new.active;
                }
            } else {
                self.home(ui);
            }
        });
        if self.active != previous_active {
            crate::slides::editor::stop_presentation(&ctx);
        }
        self.dialogs(&ctx);
    }
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        let _ = self.settings.save();
    }
}
