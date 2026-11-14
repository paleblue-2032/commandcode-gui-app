use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    Dark,
    Light,
    #[default]
    System,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionPolicy {
    #[default]
    Ask,
    AutoAllowReads,
    Yolo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub agent_binary: String,
    pub agent_args: Vec<String>,
    pub default_cwd: Option<PathBuf>,
    pub theme: ThemePreference,
    pub accent_color: [u8; 3],
    pub font_scale: f32,
    pub show_thoughts: bool,
    pub permission_policy: PermissionPolicy,
    pub window_size: [f32; 2],
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            agent_binary: "commandcode".to_string(),
            agent_args: vec!["--no-auto-update".to_string(), "acp".to_string()],
            default_cwd: None,
            theme: ThemePreference::System,
            accent_color: [232, 181, 102],
            font_scale: 1.0,
            show_thoughts: true,
            permission_policy: PermissionPolicy::Ask,
            window_size: [1280.0, 840.0],
        }
    }
}

impl Settings {
    pub fn path() -> Option<PathBuf> {
        ProjectDirs::from("", "", "commandcode-gui")
            .map(|dirs| dirs.config_dir().join("settings.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::path() else {
            return Self::default();
        };
        let Ok(content) = fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = Self::path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, content);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_accent_uses_default() {
        let settings: Settings = serde_json::from_str(r#"{"theme":"dark"}"#).unwrap();
        assert_eq!(settings.accent_color, [232, 181, 102]);
    }

    #[test]
    fn accent_round_trips() {
        let settings = Settings {
            accent_color: [124, 201, 154],
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let loaded: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.accent_color, [124, 201, 154]);
    }
}
