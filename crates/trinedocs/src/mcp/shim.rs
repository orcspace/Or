use rmcp::{
    ServerHandler, ServiceExt,
    model::*,
    service::{Peer, RequestContext, RoleClient, RoleServer},
};
// Upstream negotiates via `initialize` (pre-2026-07-28) and receives results
// without `resultType`; downstream clients may negotiate 2026-07-28, where the
// field is required, so the proxy restores the "complete" discriminator and the
// required list caching hints (`ttlMs: 0`, `cacheScope: "private"` — never cache).
#[derive(Clone)]
struct Proxy {
    peer: Peer<RoleClient>,
}
impl ServerHandler for Proxy {
    fn get_info(&self) -> ServerConfig {
        let mut info = ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        );
        if let Some(peer) = self.peer.peer_info() {
            info.capabilities = peer.capabilities.clone();
            info.protocol_version = peer.protocol_version.clone();
            if let Some(identity) = peer.server_info.clone() {
                info.server_info = identity;
            }
            info.instructions = peer.instructions.clone();
        }
        info
    }
    async fn list_tools(
        &self,
        p: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, rmcp::ErrorData> {
        self.peer
            .list_tools(p)
            .await
            .map(|mut r| {
                r.result_type.get_or_insert(ResultType::COMPLETE);
                r.ttl_ms.get_or_insert(0);
                r.cache_scope.get_or_insert(CacheScope::Private);
                r
            })
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }
    async fn call_tool(
        &self,
        r: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, rmcp::ErrorData> {
        self.peer
            .call_tool(r)
            .await
            .map(|mut r| {
                r.result_type.get_or_insert(ResultType::COMPLETE);
                r.into()
            })
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }
    async fn list_resources(
        &self,
        p: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, rmcp::ErrorData> {
        self.peer
            .list_resources(p)
            .await
            .map(|mut r| {
                r.result_type.get_or_insert(ResultType::COMPLETE);
                r.ttl_ms.get_or_insert(0);
                r.cache_scope.get_or_insert(CacheScope::Private);
                r
            })
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }
    async fn read_resource(
        &self,
        r: ReadResourceRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, rmcp::ErrorData> {
        self.peer
            .read_resource(r)
            .await
            .map(|mut r| {
                r.result_type.get_or_insert(ResultType::COMPLETE);
                r.into()
            })
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }
}
pub fn run() -> Result<(), String> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    rt.block_on(async {
        let path = crate::util::app_dir().join("mcp.json");
        let mut launched = false;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let conn = loop {
            if let Ok(bytes) = crate::util::read_limited(&path, 8192)
                && let Ok(c) = serde_json::from_slice::<super::server::Connection>(&bytes)
                && (39217..39317).contains(&c.port)
                && c.token.len() == 64
                && c.token.bytes().all(|byte| byte.is_ascii_hexdigit())
                && tokio::net::TcpStream::connect(("127.0.0.1", c.port))
                    .await
                    .is_ok()
            {
                break c;
            }
            if !launched {
                let exe = std::env::current_exe()
                    .map_err(|e| e.to_string())?
                    .with_file_name("TrineDocs.exe");
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    std::process::Command::new(exe)
                        .arg("--background")
                        .creation_flags(0x08000000)
                        .spawn()
                        .map_err(|e| e.to_string())?;
                }
                #[cfg(not(windows))]
                {
                    std::process::Command::new(exe)
                        .arg("--background")
                        .spawn()
                        .map_err(|e| e.to_string())?;
                }
                launched = true;
            }
            if std::time::Instant::now() > deadline {
                return Err("TrineDocs не запустил MCP за 10 секунд".into());
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        };
        let config =
            rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(
                format!("http://127.0.0.1:{}/mcp", conn.port),
            )
            .auth_header(conn.token);
        let transport = rmcp::transport::StreamableHttpClientTransport::with_client(
            reqwest::Client::builder()
                .no_proxy()
                .build()
                .map_err(|e| e.to_string())?,
            config,
        );
        let client = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            rmcp::model::ClientConfig::new(
                ClientCapabilities::default(),
                Implementation::new(
                    std::env::args()
                        .skip_while(|a| a != "--client")
                        .nth(1)
                        .unwrap_or_else(|| "TrineDocs stdio".into()),
                    env!("CARGO_PKG_VERSION"),
                ),
            )
            .serve(transport),
        )
        .await
        .map_err(|_| "MCP не ответил на инициализацию за 10 секунд".to_string())?
        .map_err(|e| e.to_string())?;
        let proxy = Proxy {
            peer: client.peer().clone(),
        }
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| e.to_string())?;
        proxy.waiting().await.map_err(|e| e.to_string())?;
        let _ = client.cancel().await;
        Ok(())
    })
}
