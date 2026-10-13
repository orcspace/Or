use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, ClientCapabilities, ClientConfig, Implementation},
};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
async fn call(
    peer: &rmcp::service::Peer<rmcp::service::RoleClient>,
    name: &'static str,
    args: Value,
) -> Result<Value, String> {
    let params = CallToolRequestParams::new(name)
        .with_arguments(args.as_object().cloned().ok_or("Tool args")?);
    let result = peer.call_tool(params).await.map_err(|e| e.to_string())?;
    if result.is_error == Some(true) {
        return Err(format!("MCP {name}: {:?}", result.content));
    }
    result
        .structured_content
        .ok_or_else(|| format!("MCP {name}: no structured result"))
}
pub fn run(root: &Path) -> Result<(), String> {
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let config = root.join("appdata/TrineDocs/mcp.json");
    let (handle, rx) = super::server::Handle::start_at(egui::Context::default(), config);
    let stats = Arc::new(Mutex::new(Vec::<String>::new()));
    let capture = stats.clone();
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let pump = std::thread::spawn(move || {
        let mut tabs = Vec::new();
        let mut next = 1;
        while !stopped.load(Ordering::Relaxed) {
            match rx.recv_timeout(Duration::from_millis(20)) {
                Ok(r) => {
                    if let Ok(mut v) = capture.lock() {
                        v.push(r.client.clone());
                    }
                    let result =
                        super::tools::dispatch(&mut tabs, &mut next, &r.tool, &r.args, &r.client);
                    let _ = r.reply.send(result);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => break,
            }
        }
    });
    let result = (|| {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let connection = loop {
            if let Ok(s) = handle.connection.lock()
                && let Ok(c) = &*s
            {
                break c.clone();
            }
            if std::time::Instant::now() > deadline {
                return Err("MCP did not start".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        rt.block_on(async{
let http=reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(5)).build().map_err(|e|e.to_string())?;let url=format!("http://127.0.0.1:{}/mcp",connection.port);
if http.get(&url).send().await.map_err(|e|e.to_string())?.status()!=reqwest::StatusCode::UNAUTHORIZED{return Err("Unauthenticated request accepted".into());}
for(header,value)in[("Origin","https://evil.invalid"),("Host","evil.invalid")]{let response=http.post(&url).bearer_auth(&connection.token).header(header,value).header("Accept","application/json, text/event-stream").json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2026-07-28","capabilities":{},"clientInfo":{"name":"probe","version":"1"}}})).send().await.map_err(|e|e.to_string())?;if response.status().is_success(){return Err(format!("{header} validation failed"));}}
let response=http.post(&url).bearer_auth(&connection.token).header("Accept","application/json, text/event-stream").header("Content-Type","application/json").body(vec![b' ';1024*1024+1]).send().await.map_err(|e|e.to_string())?;if response.status().is_success(){return Err("Oversized request accepted".into());}
let config=rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(url).auth_header(connection.token);
let transport=rmcp::transport::StreamableHttpClientTransport::with_client(http,config);
let client=ClientConfig::new(ClientCapabilities::default(),Implementation::new("Smoke HTTP","1")).serve(transport).await.map_err(|e|e.to_string())?;
let tools=client.list_all_tools().await.map_err(|e|e.to_string())?;if tools.len()<50{return Err("Missing MCP tools".into());}
let doc=call(client.peer(),"create_document",json!({"kind":"docs"})).await?;let id=doc.get("document_id").and_then(Value::as_u64).ok_or("No document ID")?;
let content=call(client.peer(),"docs_read",json!({"document_id":id})).await?;let paragraph=content.pointer("/content/paragraphs/0/id").and_then(Value::as_u64).ok_or("No paragraph ID")?;
call(client.peer(),"docs_insert_text",json!({"document_id":id,"paragraph_id":paragraph,"text":"Agent: Привет"})).await?;
call(client.peer(),"undo",json!({"document_id":id})).await?;
call(client.peer(),"redo",json!({"document_id":id})).await?;
let shim=std::env::current_exe().map_err(|e|e.to_string())?.with_file_name("trinedocs-mcp.exe");if !shim.exists(){return Err("Build the stdio shim before MCP smoke".into());}
let mut cmd=tokio::process::Command::new(shim);cmd.kill_on_drop(true).env("APPDATA",root.join("appdata")).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null());#[cfg(windows)]cmd.creation_flags(0x08000000);
let mut child=cmd.spawn().map_err(|e|e.to_string())?;let stdout=child.stdout.take().ok_or("No stdout")?;let stdin=child.stdin.take().ok_or("No stdin")?;
let shim_result=async {let stdio=ClientConfig::new(ClientCapabilities::default(),Implementation::new("Smoke stdio","1")).serve((stdout,stdin)).await.map_err(|e|e.to_string())?;
let result=call(stdio.peer(),"docs_read",json!({"document_id":id})).await?;if result.pointer("/content/paragraphs/0/runs/0/text").and_then(Value::as_str)!=Some("Agent: Привет"){return Err("Shim read mismatch".into());}
let _=stdio.cancel().await;Ok::<(),String>(())}.await;let _=child.kill().await;let _=child.wait().await;shim_result?;let _=client.cancel().await;Ok(())
})
    })();
    stop.store(true, Ordering::Relaxed);
    drop(handle);
    let _ = pump.join();
    result?;
    let clients = stats.lock().map_err(|e| e.to_string())?;
    if clients.len() < 6
        || clients[0] != clients[1]
        || clients[0] == *clients.last().ok_or("No clients")?
    {
        return Err("MCP session identity is unstable".into());
    }
    crate::util::atomic_write(&root.join("result.json"),br#"{"status":"passed","http":true,"stdio":true,"auth":true,"host_origin":true,"request_limit":true,"session_identity":true}"#,false)
}
