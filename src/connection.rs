//! Single-session connection configuration.
//!
//! Replaces sessions_config.rs / session_manager.rs.
//! Stores one connection entry in ~/.vellum-fe/connection.toml.
//! Passwords are never stored here — they live in the OS keychain.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum ConnectionConfig {
    /// Connect via Lich proxy (no auth needed — Lich handles it)
    Lich {
        #[serde(default = "default_host")]
        host: String,
        #[serde(default = "default_port")]
        port: u16,
    },
    /// Connect directly via eAccess SGE
    Direct {
        account: String,
        character: String,
        #[serde(default = "default_game_code")]
        game_code: String,
    },
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}
fn default_port() -> u16 {
    8000
}
fn default_game_code() -> String {
    "GS3".to_string()
}

impl ConnectionConfig {
    pub fn load() -> Result<Option<Self>> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(None);
        }
        let s = std::fs::read_to_string(&path).with_context(|| format!("reading {:?}", path))?;
        let cfg = toml::from_str(&s).with_context(|| format!("parsing {:?}", path))?;
        Ok(Some(cfg))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        let s = toml::to_string_pretty(self).context("serializing connection config")?;
        std::fs::write(&path, s).with_context(|| format!("writing {:?}", path))
    }

    fn path() -> Result<PathBuf> {
        let base = crate::config::Config::base_dir()?;
        Ok(base.join("connection.toml"))
    }
}
