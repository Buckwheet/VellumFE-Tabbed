//! Persistent session list stored in sessions.toml.
//!
//! Saves/loads the list of configured sessions so they survive restarts.
//! Passwords are never stored here.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionModeConfig {
    Lich,
    Direct,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    /// Display name shown in tab bar
    pub label: String,
    /// Connection mode
    pub mode: SessionModeConfig,
    /// Lich proxy host (mode = lich)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// Lich proxy port (mode = lich)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// Account name (mode = direct)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// Character name (mode = direct)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character: Option<String>,
    /// Game code e.g. GS3, GSX (mode = direct)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_code: Option<String>,
    /// Whether to connect automatically on startup
    #[serde(default)]
    pub auto_connect: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SessionsConfig {
    #[serde(default)]
    pub sessions: Vec<SessionEntry>,
}

impl SessionsConfig {
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {:?}", path))?;
        toml::from_str(&contents)
            .with_context(|| format!("Failed to parse {:?}", path))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize sessions config")?;
        fs::write(&path, contents)
            .with_context(|| format!("Failed to write {:?}", path))
    }

    pub fn add(&mut self, entry: SessionEntry) {
        self.sessions.push(entry);
    }

    pub fn remove(&mut self, label: &str) {
        self.sessions.retain(|s| s.label != label);
    }

    fn path() -> Result<PathBuf> {
        let base = dirs::config_dir()
            .context("Could not determine config directory")?;
        Ok(base.join("vellum-fe-tabbed").join("sessions.toml"))
    }
}