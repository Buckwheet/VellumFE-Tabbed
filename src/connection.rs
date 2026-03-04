//! Multi-profile connection configuration — Warlock-style login manager.
//!
//! Stores named character profiles in ~/.vellum-fe/profiles.toml.
//! Passwords are never stored here — they live in the OS keychain.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileStore {
    #[serde(default)]
    pub profiles: Vec<Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub account: String,
    pub character: String,
    #[serde(default = "default_game_code")]
    pub game_code: String,
    #[serde(default)]
    pub use_lich: bool,
    pub lich_host: Option<String>,
    pub lich_port: Option<u16>,
}

fn default_game_code() -> String {
    "GS3".to_string()
}

impl Profile {
    pub fn lich_host(&self) -> &str {
        self.lich_host.as_deref().unwrap_or("127.0.0.1")
    }
    pub fn lich_port(&self) -> u16 {
        self.lich_port.unwrap_or(8000)
    }
}

impl ProfileStore {
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let s = std::fs::read_to_string(&path).with_context(|| format!("reading {:?}", path))?;
        let store = toml::from_str(&s).with_context(|| format!("parsing {:?}", path))?;
        Ok(store)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        let s = toml::to_string_pretty(self).context("serializing profiles")?;
        std::fs::write(&path, s).with_context(|| format!("writing {:?}", path))
    }

    pub fn path() -> Result<PathBuf> {
        let base = crate::config::Config::base_dir().context("could not determine config dir")?;
        Ok(base.join("profiles.toml"))
    }

    pub fn add_or_update(&mut self, profile: Profile) {
        if let Some(existing) = self.profiles.iter_mut().find(|p| p.name == profile.name) {
            *existing = profile;
        } else {
            self.profiles.push(profile);
        }
    }

    pub fn remove(&mut self, name: &str) {
        self.profiles.retain(|p| p.name != name);
    }
}
