use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub poll_interval_secs: u64,
    pub sound_enabled: bool,
    pub sound_path: Option<String>,
    pub auto_launch: bool,
    pub gmail_query: String,
    pub oauth_client_id: String,
    pub oauth_client_secret: Option<String>,
    pub playback_volume: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            poll_interval_secs: 60,
            sound_enabled: true,
            sound_path: None,
            auto_launch: true,
            gmail_query: "is:unread category:primary".to_string(),
            oauth_client_id: String::new(),
            oauth_client_secret: None,
            playback_volume: 0.7,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SettingsUpdate {
    pub poll_interval_secs: Option<u64>,
    pub sound_enabled: Option<bool>,
    pub sound_path: Option<Option<String>>,
    pub auto_launch: Option<bool>,
    pub gmail_query: Option<String>,
    pub oauth_client_id: Option<String>,
    pub oauth_client_secret: Option<Option<String>>,
    pub playback_volume: Option<f32>,
}

pub struct SettingsManager {
    path: PathBuf,
    state: RwLock<Settings>,
}

impl SettingsManager {
    pub fn initialize(app: &AppHandle) -> Result<Self> {
        let path = ensure_settings_path(app)?;
        let settings = load_settings(&path)?;
        Ok(Self {
            path,
            state: RwLock::new(settings),
        })
    }

    pub fn get(&self) -> Settings {
        self.state.read().clone()
    }

    pub fn update(&self, update: SettingsUpdate) -> Result<Settings> {
        let mut guard = self.state.write();
        if let Some(value) = update.poll_interval_secs {
            guard.poll_interval_secs = value.clamp(15, 300);
        }
        if let Some(value) = update.sound_enabled {
            guard.sound_enabled = value;
        }
        if let Some(value) = update.sound_path {
            guard.sound_path = value;
        }
        if let Some(value) = update.auto_launch {
            guard.auto_launch = value;
        }
        if let Some(value) = update.gmail_query {
            guard.gmail_query = value;
        }
        if let Some(value) = update.oauth_client_id {
            guard.oauth_client_id = value;
        }
        if let Some(value) = update.oauth_client_secret {
            guard.oauth_client_secret = value;
        }
        if let Some(value) = update.playback_volume {
            guard.playback_volume = value.clamp(0.0, 1.0);
        }
        save_settings(&self.path, &guard)?;
        Ok(guard.clone())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn ensure_settings_path(_app: &AppHandle) -> Result<PathBuf> {
    // Use directories crate to determine a config directory compatible with Tauri v2
    let proj = ProjectDirs::from("org", "kreditpro", "GmailTrayNotifier")
        .context("Unable to resolve configuration directory")?;
    let mut dir = proj.config_dir().to_path_buf();
    if !dir.exists() {
        fs::create_dir_all(&dir).context("Failed to create configuration directory")?;
    }
    dir.push("settings.json");
    Ok(dir)
}

fn load_settings(path: &Path) -> Result<Settings> {
    if !path.exists() {
        let defaults = Settings::default();
        save_settings(path, &defaults)?;
        return Ok(defaults);
    }
    let bytes = fs::read(path).context("Failed to read settings file")?;
    let settings: Settings = serde_json::from_slice(&bytes).context("Invalid settings file")?;
    Ok(settings)
}

fn save_settings(path: &Path, settings: &Settings) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Failed to create parent directory")?;
    }
    let json = serde_json::to_vec_pretty(settings).context("Failed to serialise settings")?;
    fs::write(path, json).context("Failed to write settings file")?;
    Ok(())
}
