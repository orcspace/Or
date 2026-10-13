use serde_json::json;
use std::path::{Path, PathBuf};
#[derive(Clone, Copy, Debug)]
pub enum Client {
    Codex,
    Claude,
    Cursor,
    VsCode,
}
impl Client {
    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude Desktop",
            Self::Cursor => "Cursor",
            Self::VsCode => "VS Code",
        }
    }
    pub fn path(self) -> Result<PathBuf, String> {
        let home = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .ok_or("Нет USERPROFILE")?;
        let app = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .ok_or("Нет APPDATA")?;
        Ok(match self {
            Self::Codex => std::env::var_os("CODEX_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".codex"))
                .join("config.toml"),
            Self::Claude => app.join("Claude/claude_desktop_config.json"),
            Self::Cursor => home.join(".cursor/mcp.json"),
            Self::VsCode => app.join("Code/User/mcp.json"),
        })
    }
}
#[derive(Clone)]
pub struct Preview {
    pub path: PathBuf,
    pub change: String,
    before: Option<Vec<u8>>,
    after: String,
}
pub fn preview(client: Client, shim: &Path) -> Result<Preview, String> {
    prepare(client, client.path()?, shim)
}
fn prepare(client: Client, path: PathBuf, shim: &Path) -> Result<Preview, String> {
    if !shim.is_absolute() {
        return Err("Путь шима должен быть абсолютным".into());
    }
    let before = if path.exists() {
        Some(crate::util::read_limited(&path, 1024 * 1024)?)
    } else {
        None
    };
    let text = before
        .as_deref()
        .map(std::str::from_utf8)
        .transpose()
        .map_err(|e| e.to_string())?
        .unwrap_or("");
    let command = shim.to_str().ok_or("Путь шима не UTF-8")?;
    let (after, change) = if matches!(client, Client::Codex) {
        let mut doc = text
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| format!("Конфиг TOML не изменён: {e}"))?;
        if doc.get("mcp_servers").is_some_and(|i| !i.is_table()) {
            return Err("Раздел mcp_servers должен быть обычной TOML таблицей; inline/scalar конфиги требуют ручного добавления".into());
        }
        let entry = &mut doc["mcp_servers"]["trinedocs"];
        if !entry.is_none() {
            return Err("Запись trinedocs уже существует; отредактируйте её вручную".into());
        }
        *entry = toml_edit::Item::Table(toml_edit::Table::new());
        entry["command"] = toml_edit::value(command);
        let mut args = toml_edit::Array::new();
        args.push("--client");
        args.push(client.label());
        entry["args"] = toml_edit::value(args);
        (
            doc.to_string(),
            format!(
                "[mcp_servers.trinedocs]\ncommand = {}\nargs = [\"--client\", {}]",
                serde_json::to_string(command).map_err(|e| e.to_string())?,
                serde_json::to_string(client.label()).map_err(|e| e.to_string())?
            ),
        )
    } else {
        let mut doc: serde_json::Value = if text.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(text).map_err(|e|format!("Конфиг JSON/JSONC не изменён: {e}. Комментарии требуют ручного добавления записи."))?
        };
        let key = if matches!(client, Client::VsCode) {
            "servers"
        } else {
            "mcpServers"
        };
        let root = doc
            .as_object_mut()
            .ok_or("Корень конфигурации должен быть объектом")?;
        let servers = root
            .entry(key)
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or("Раздел серверов должен быть объектом")?;
        if servers.contains_key("trinedocs") {
            return Err("Запись trinedocs уже существует; отредактируйте её вручную".into());
        }
        let entry = if matches!(client, Client::VsCode) {
            json!({"type":"stdio","command":command,"args":["--client",client.label()]})
        } else {
            json!({"command":command,"args":["--client",client.label()]})
        };
        servers.insert("trinedocs".into(), entry.clone());
        (
            serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?,
            serde_json::to_string_pretty(&json!({key:{"trinedocs":entry}}))
                .map_err(|e| e.to_string())?,
        )
    };
    Ok(Preview {
        path,
        change,
        before,
        after,
    })
}
pub fn apply(p: &Preview) -> Result<(), String> {
    let current = if p.path.exists() {
        Some(crate::util::read_limited(&p.path, 1024 * 1024)?)
    } else {
        None
    };
    if current != p.before {
        return Err(
            "Конфигурация изменилась после предпросмотра; откройте новый предпросмотр".into(),
        );
    }
    crate::util::atomic_write(&p.path, p.after.as_bytes(), true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registration_requires_absolute_shim_path() {
        assert!(
            prepare(
                Client::Codex,
                std::env::temp_dir().join("unused-config.toml"),
                Path::new("trinedocs-mcp.exe")
            )
            .is_err()
        );
    }
    #[test]
    fn preserves_unrelated_toml_and_rejects_stale_preview() -> Result<(), String> {
        let path =
            std::env::temp_dir().join(format!("trinedocs-register-{}.toml", std::process::id()));
        crate::util::atomic_write(
            &path,
            b"# keep\nmodel = 'example'\n[mcp_servers.other]\ncommand = 'x'\n",
            false,
        )?;
        let p = prepare(
            Client::Codex,
            path.clone(),
            Path::new("C:/TrineDocs/trinedocs-mcp.exe"),
        )?;
        assert!(p.after.contains("# keep"));
        assert!(p.after.contains("mcp_servers.other"));
        assert!(p.after.contains("mcp_servers.trinedocs"));
        crate::util::atomic_write(&path, b"model='changed'", false)?;
        assert!(apply(&p).is_err());
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
        Ok(())
    }
}
