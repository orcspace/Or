use crate::{native::Document, ops::Tab};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    time::{Duration, Instant},
};

pub const MAX_JOBS: usize = 4;
pub const MAX_DOCUMENTS: usize = 4;
pub const MAX_OPERATIONS: usize = 32;
pub const TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub tool: String,
    pub arguments: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    operations: Vec<Operation>,
    #[serde(default = "apply_mode")]
    mode: String,
}
fn apply_mode() -> String {
    "apply".into()
}
pub struct Plan {
    pub groups: BTreeMap<u64, (u64, Vec<Operation>)>,
    pub dry_run: bool,
}
pub fn batchable(name: &str) -> bool {
    super::tools::names().contains(&name)
        && !super::tools::read_only(name)
        && !matches!(
            name,
            "create_document"
                | "open_document"
                | "close_document"
                | "save_document"
                | "export"
                | "undo"
                | "redo"
                | "set_selection"
                | "workspace_apply"
                | "workspace_cancel"
        )
}
pub fn parse(arguments: &Value) -> Result<Plan, String> {
    struct Limit(usize);
    impl std::io::Write for Limit {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_sub(bytes.len())
                .ok_or_else(|| std::io::Error::other("limit"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Limit(1024 * 1024), arguments)
        .map_err(|_| "too_large: пакет MCP превышает 1 МБ")?;
    let request: Request =
        serde_json::from_value(arguments.clone()).map_err(|e| format!("invalid_args: {e}"))?;
    if request.operations.is_empty() || request.operations.len() > MAX_OPERATIONS {
        return Err("invalid_args: нужно от 1 до 32 операций".into());
    }
    if !matches!(request.mode.as_str(), "apply" | "dry_run") {
        return Err("invalid_args: mode apply/dry_run".into());
    }
    let mut groups = BTreeMap::<u64, (u64, Vec<Operation>)>::new();
    for operation in request.operations {
        if !batchable(&operation.tool) {
            return Err(format!(
                "unsupported_feature: {} нельзя включать в пакет",
                operation.tool
            ));
        }
        super::tools::validate_args(&operation.tool, &operation.arguments)?;
        if operation.arguments.get("client_access").is_some() {
            return Err("invalid_args: client_access разрешён только для всего вызова, не внутри operations".into());
        }
        if operation.arguments.get("mode").is_some() {
            return Err("invalid_args: mode задаётся для всего пакета".into());
        }
        let id = operation
            .arguments
            .get("document_id")
            .and_then(Value::as_u64)
            .filter(|id| *id != 0)
            .ok_or("invalid_args: каждой операции нужен document_id")?;
        let version = operation
            .arguments
            .get("base_version")
            .and_then(Value::as_u64)
            .ok_or("invalid_args: каждой операции нужен base_version исходного снимка")?;
        let group = groups.entry(id).or_insert_with(|| (version, Vec::new()));
        if group.0 != version {
            return Err(
                "invalid_args: все операции документа используют одну исходную base_version".into(),
            );
        }
        group.1.push(operation);
    }
    if groups.len() > MAX_DOCUMENTS {
        return Err("too_large: не более 4 документов на пакет".into());
    }
    Ok(Plan {
        groups,
        dry_run: request.mode == "dry_run",
    })
}
pub struct Snapshot {
    pub id: u64,
    pub title: String,
    pub version: u64,
    pub document: Document,
}
pub struct Task {
    pub id: String,
    pub client: String,
    pub targets: Vec<(u64, u64, String)>,
    pub dry_run: bool,
    pub cues: BTreeMap<u64, Value>,
    pub started: Instant,
    cancel: Arc<AtomicBool>,
    receiver: Option<Receiver<Result<Vec<crate::ops::WorkspaceUpdate>, String>>>,
    status: &'static str,
    result: Option<Value>,
    error: Option<String>,
}
impl Task {
    pub fn start(
        plan: Plan,
        snapshots: Vec<Snapshot>,
        client: String,
        ctx: egui::Context,
    ) -> Result<Self, String> {
        let mut nonce = [0_u8; 16];
        getrandom::fill(&mut nonce).map_err(|e| e.to_string())?;
        let id = format!(
            "job-{}",
            nonce.iter().map(|b| format!("{b:02x}")).collect::<String>()
        );
        let targets = snapshots
            .iter()
            .map(|s| (s.id, s.version, s.title.clone()))
            .collect();
        let cues = plan
            .groups
            .iter()
            .filter_map(|(&id, (_, operations))| {
                operations
                    .first()
                    .map(|operation| (id, operation.arguments.clone()))
            })
            .collect();
        let dry_run = plan.dry_run;
        let cancel = Arc::new(AtomicBool::new(false));
        let owner_cancel = cancel.clone();
        let started = Instant::now();
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker_client = client.clone();
        std::thread::Builder::new()
            .name("workspace-plan".into())
            .spawn(move || {
                let result = prepare(plan, snapshots, &worker_client, &owner_cancel, started);
                let _ = sender.send(result);
                ctx.request_repaint();
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            id,
            client,
            targets,
            dry_run,
            cues,
            started,
            cancel,
            receiver: Some(receiver),
            status: "preparing",
            result: None,
            error: None,
        })
    }
    #[cfg(test)]
    pub(crate) fn ready_for_test(client: &str, updates: Vec<crate::ops::WorkspaceUpdate>) -> Self {
        let targets = updates
            .iter()
            .map(|u| (u.id, u.base_version, "Test".into()))
            .collect();
        let (tx, receiver) = mpsc::sync_channel(1);
        let _ = tx.send(Ok(updates));
        Self {
            id: "job-00000000000000000000000000000000".into(),
            client: client.into(),
            targets,
            dry_run: false,
            cues: BTreeMap::new(),
            started: Instant::now(),
            cancel: Arc::new(AtomicBool::new(false)),
            receiver: Some(receiver),
            status: "preparing",
            result: None,
            error: None,
        }
    }
    pub fn state(&self) -> &'static str {
        self.status
    }
    pub fn running(&self) -> bool {
        self.receiver.is_some()
    }
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
    pub fn cancel(&mut self) {
        if self.running() {
            self.cancel.store(true, Ordering::Relaxed);
            self.status = "cancelled";
        }
    }
    pub fn poll(&mut self) -> Option<Result<Vec<crate::ops::WorkspaceUpdate>, String>> {
        let result = match self.receiver.as_ref()?.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Err("internal: подготовка пакета прервана".into())
            }
        };
        self.receiver = None;
        Some(result)
    }
    pub fn complete(&mut self, result: Value) {
        self.status = if self.dry_run { "preview" } else { "completed" };
        self.result = Some(result);
    }
    pub fn fail(&mut self, error: String) {
        self.status = if self.cancelled() {
            "cancelled"
        } else {
            "failed"
        };
        self.error = Some(error);
    }
    pub fn status(&self) -> Value {
        json!({
            "job_id":self.id, "status":self.status, "mode":if self.dry_run {"dry_run"} else {"apply"},
            "documents":self.targets.iter().map(|(id,version,title)| json!({"document_id":id,"base_version":version,"title":title})).collect::<Vec<_>>(),
            "result":self.result, "error":self.error,
        })
    }
}
impl Drop for Task {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
fn prepare(
    mut plan: Plan,
    snapshots: Vec<Snapshot>,
    client: &str,
    cancel: &Arc<AtomicBool>,
    started: Instant,
) -> Result<Vec<crate::ops::WorkspaceUpdate>, String> {
    std::thread::scope(|scope| {
        let mut workers = Vec::new();
        for snapshot in snapshots {
            let (_, operations) = plan
                .groups
                .remove(&snapshot.id)
                .ok_or("internal: отсутствует план документа")?;
            let cancel = cancel.clone();
            let client = client.to_owned();
            let worker = std::thread::Builder::new()
                .name(format!("workspace-doc-{}", snapshot.id))
                .spawn_scoped(scope, move || {
                    prepare_document(snapshot, operations, &client, &cancel, started)
                })
                .map_err(|e| e.to_string())?;
            workers.push(worker);
        }
        let mut updates = Vec::new();
        for worker in workers {
            updates.push(
                worker
                    .join()
                    .map_err(|_| "internal: поток подготовки документа прерван")??,
            );
        }
        if !plan.groups.is_empty() {
            return Err("internal: неполные снимки пакета".into());
        }
        Ok(updates)
    })
}
fn prepare_document(
    snapshot: Snapshot,
    operations: Vec<Operation>,
    client: &str,
    cancel: &AtomicBool,
    started: Instant,
) -> Result<crate::ops::WorkspaceUpdate, String> {
    let mut tab = Tab::new(snapshot.id, snapshot.document, snapshot.title, None);
    tab.version = snapshot.version;
    tab.saved_version = snapshot.version;
    let mut tabs = vec![tab];
    let mut next_id = 1;
    let mut changed_ids = Vec::new();
    for operation in operations {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled: пакет отменён".into());
        }
        if started.elapsed() > TIMEOUT {
            return Err("timeout: подготовка превысила 60 секунд".into());
        }
        let mut arguments = operation.arguments;
        let map = arguments
            .as_object_mut()
            .ok_or("invalid_args: arguments должен быть объектом")?;
        map.insert("base_version".into(), json!(tabs[0].version));
        let result =
            super::tools::dispatch(&mut tabs, &mut next_id, &operation.tool, &arguments, client)?;
        if let Some(ids) = result.get("changed_ids").and_then(Value::as_array) {
            changed_ids.extend(ids.iter().filter_map(Value::as_u64));
        }
        // Worker-only history is not the user's history. Keep bounded transient state.
        tabs[0].undo.clear();
        tabs[0].ai.clear();
        tabs[0].pending_docs_edits = Some(Vec::new());
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled: пакет отменён".into());
    }
    changed_ids.sort_unstable();
    changed_ids.dedup();
    let tab = tabs
        .pop()
        .ok_or("internal: документ подготовки отсутствует")?;
    Ok(crate::ops::WorkspaceUpdate {
        id: snapshot.id,
        base_version: snapshot.version,
        document: tab.document,
        changed_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plan_requires_consistent_explicit_versions_and_refuses_disk_or_recursive_calls() {
        for arguments in [
            json!({"operations":[]}),
            json!({"operations":[{"tool":"docs_insert_text","arguments":{"document_id":1}}]}),
            json!({"operations":[{"tool":"save_document","arguments":{"document_id":1,"base_version":0}}]}),
            json!({"operations":[{"tool":"workspace_apply","arguments":{}}]}),
            json!({"operations":[
                {"tool":"slides_set_notes","arguments":{"document_id":1,"base_version":0,"slide_id":1,"notes":"a"}},
                {"tool":"slides_set_notes","arguments":{"document_id":1,"base_version":1,"slide_id":1,"notes":"b"}}
            ]}),
        ] {
            assert!(parse(&arguments).is_err());
        }
    }
    #[test]
    fn independent_documents_prepare_without_changing_their_source_snapshots() -> Result<(), String>
    {
        let docs = crate::docs::model::Doc::from_text("Start");
        let paragraph = docs.paragraphs[0].id;
        let deck = crate::slides::model::Deck::default();
        let slide = deck.slides[0].id;
        let plan = parse(&json!({"operations":[
            {"tool":"docs_insert_text","arguments":{"document_id":10,"base_version":4,"paragraph_id":paragraph,"offset":5,"text":" done"}},
            {"tool":"slides_set_notes","arguments":{"document_id":20,"base_version":9,"slide_id":slide,"notes":"Prepared"}}
        ]}))?;
        let snapshots = vec![
            Snapshot {
                id: 10,
                title: "Doc".into(),
                version: 4,
                document: Document::Docs(docs.clone()),
            },
            Snapshot {
                id: 20,
                title: "Deck".into(),
                version: 9,
                document: Document::Slides(deck.clone()),
            },
        ];
        let updates = prepare(
            plan,
            snapshots,
            "agent",
            &Arc::new(AtomicBool::new(false)),
            Instant::now(),
        )?;
        assert_eq!(updates.len(), 2);
        assert!(
            matches!(&updates[0].document, Document::Docs(doc) if doc.plain_text()=="Start done")
        );
        assert!(
            matches!(&updates[1].document, Document::Slides(deck) if deck.slides[0].notes=="Prepared")
        );
        assert_eq!(docs.plain_text(), "Start");
        assert_eq!(deck.slides[0].notes, "");
        Ok(())
    }
    #[test]
    fn cancellation_stops_before_any_prepared_edit() -> Result<(), String> {
        let deck = crate::slides::model::Deck::default();
        let slide = deck.slides[0].id;
        let plan = parse(
            &json!({"operations":[{"tool":"slides_set_notes","arguments":{"document_id":1,"base_version":0,"slide_id":slide,"notes":"Changed"}}]}),
        )?;
        let snapshots = vec![Snapshot {
            id: 1,
            title: "Deck".into(),
            version: 0,
            document: Document::Slides(deck),
        }];
        assert!(
            prepare(
                plan,
                snapshots,
                "agent",
                &Arc::new(AtomicBool::new(true)),
                Instant::now()
            )
            .is_err()
        );
        Ok(())
    }
}
