use serde::{Deserialize, Serialize};
use std::path::PathBuf;
const MAX_SETTINGS: usize = 1024 * 1024;
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
                crate::util::read_limited(candidate, MAX_SETTINGS)
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
        let mut recent = std::collections::HashSet::new();
        self.recent.retain(|path| recent.insert(path.clone()));
        let mut pinned = std::collections::HashSet::new();
        self.pinned.retain(|path| pinned.insert(path.clone()));
        self.recent.truncate(50);
        self.pinned.truncate(50);
        self.chat_models = self.chat_models.map(|m| crate::chat::clean_model(&m));
        self.chat_efforts = self.chat_efforts.map(|e| crate::chat::clean_effort(&e));
        self
    }
    fn encoded(&self) -> Result<Vec<u8>, String> {
        let bytes = serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?;
        if bytes.len() > MAX_SETTINGS {
            return Err("Настройки превышают1МиБ; сократите список длинных путей. Предыдущий файл сохранён.".into());
        }
        Ok(bytes)
    }
    pub fn save(&self) -> Result<(), String> {
        let bytes = self.encoded()?;
        crate::util::atomic_write(&crate::util::app_dir().join("settings.json"), &bytes, true)
    }
    pub fn toggle_pin(&mut self, path: PathBuf) -> Result<(), String> {
        if self.pinned.contains(&path) {
            self.pinned.retain(|p| p != &path);
        } else if self.pinned.len() >= 50 {
            return Err("Можно закрепить до 50 файлов. Сначала открепите один из них.".into());
        } else {
            self.pinned.push(path);
        }
        Ok(())
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

    #[test]
    fn pins_do_not_disappear_due_to_a_different_limit_after_restart() -> Result<(), String> {
        let mut settings = super::Settings::default();
        for index in 0..50 {
            settings.toggle_pin(format!("C:/pin-{index}.docx").into())?;
        }
        let before = settings.pinned.clone();
        assert!(settings.toggle_pin("C:/extra.docx".into()).is_err());
        assert_eq!(settings.pinned, before);
        let reloaded: super::Settings =
            serde_json::from_slice(&serde_json::to_vec(&settings).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        assert_eq!(reloaded.normalized().pinned, before);
        settings.toggle_pin(before[0].clone())?;
        settings.toggle_pin("C:/extra.docx".into())?;
        assert_eq!(settings.pinned.len(), 50);
        Ok(())
    }
}

#[cfg(test)]
mod recent_defect_tests {
    use super::*;
    #[test]
    fn duplicate_settings_paths_do_not_displace_unique_recent_files() {
        let mut settings = Settings::default();
        settings.recent = vec![PathBuf::from("C:/a.docx"); 50];
        settings.recent.push("C:/b.docx".into());
        settings.pinned = vec!["C:/a.docx".into(), "C:/a.docx".into()];
        let settings = settings.normalized();
        assert_eq!(
            settings.recent,
            vec![PathBuf::from("C:/a.docx"), PathBuf::from("C:/b.docx")]
        );
        assert_eq!(settings.pinned.len(), 1);
    }
}

#[cfg(test)]
mod two_hundred_settings_regressions {
    use super::*;
    #[test]
    fn saved_settings_cannot_exceed_their_own_reload_budget() -> Result<(), String> {
        let mut settings = Settings::default();
        settings.recent = (0..50)
            .map(|index| PathBuf::from(format!("C:/{index}/{}.docx", "x/".repeat(12000))))
            .collect();
        assert!(settings.clone().normalized().encoded().is_err());
        settings.recent.truncate(1);
        let bytes = settings.encoded()?;
        assert!(bytes.len() <= MAX_SETTINGS);
        let decoded: Settings =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        assert_eq!(decoded.normalized().recent, settings.recent);
        Ok(())
    }
}
