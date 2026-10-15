use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: String,
    pub theme: String,
    pub scale: f32,
    pub mcp: bool,
    pub dark_paper: bool,
    pub reduced_motion: bool,
    pub backup: bool,
    pub recent: Vec<PathBuf>,
    pub pinned: Vec<PathBuf>,
    /// AI chat model and effort per CLI (`[claude, codex]`); empty = the CLI default.
    pub chat_models: [String; 2],
    pub chat_efforts: [String; 2],
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "ru".into(),
            theme: "light".into(),
            scale: 1.0,
            mcp: true,
            dark_paper: false,
            reduced_motion: false,
            backup: true,
            recent: Vec::new(),
            pinned: Vec::new(),
            chat_models: Default::default(),
            chat_efforts: Default::default(),
        }
    }
}
impl Settings {
    pub fn load() -> Self {
        Self::load_at(&crate::util::app_dir().join("settings.json"))
    }
    fn load_at(path: &std::path::Path) -> Self {
        [path.to_path_buf(), path.with_extension("json.bak")]
            .iter()
            .find_map(|candidate| {
                crate::util::read_limited(candidate, 1024 * 1024)
                    .ok()
                    .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
                    .map(Self::normalized)
            })
            .unwrap_or_default()
    }
    fn normalized(mut self) -> Self {
        if !self.scale.is_finite() {
            self.scale = 1.0;
        }
        self.scale = self.scale.clamp(0.75, 2.0);
        if !matches!(self.language.as_str(), "ru" | "en") {
            self.language = "ru".into();
        }
        if !matches!(self.theme.as_str(), "light" | "dark" | "system") {
            self.theme = "system".into();
        }
        self.recent.truncate(50);
        self.pinned.truncate(50);
        self.chat_models = self.chat_models.map(|m| crate::chat::clean_model(&m));
        self.chat_efforts = self.chat_efforts.map(|e| crate::chat::clean_effort(&e));
        self
    }
    pub fn save(&self) -> Result<(), String> {
        crate::util::atomic_write(
            &crate::util::app_dir().join("settings.json"),
            &serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
            true,
        )
    }
    pub fn remember(&mut self, path: PathBuf) {
        self.recent.retain(|p| p != &path);
        self.recent.insert(0, path);
        self.recent.truncate(50);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn settings_bounds() {
        let s = super::Settings {
            scale: 0.0,
            language: "bad".into(),
            theme: "bad".into(),
            ..Default::default()
        }
        .normalized();
        assert_eq!(s.scale, 0.75);
        assert_eq!(s.language, "ru");
        assert_eq!(s.theme, "system");
        assert_eq!(
            super::Settings {
                scale: f32::NAN,
                ..Default::default()
            }
            .normalized()
            .scale,
            1.0
        );
    }
    #[test]
    fn corrupted_primary_settings_fall_back_to_the_backup() -> Result<(), String> {
        let root = std::env::temp_dir().join(format!(
            "TrineDocs-settings-backup-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let path = root.join("settings.json");
        std::fs::write(&path, b"{invalid").map_err(|e| e.to_string())?;
        std::fs::write(
            path.with_extension("json.bak"),
            br#"{"language":"en","recent":["C:/document.docx"]}"#,
        )
        .map_err(|e| e.to_string())?;
        let settings = super::Settings::load_at(&path);
        assert_eq!(settings.language, "en");
        assert_eq!(settings.recent.len(), 1);
        std::fs::remove_dir_all(root).map_err(|e| e.to_string())?;
        Ok(())
    }
}
