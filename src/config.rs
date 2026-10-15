//! Settings Hidra keeps on this computer, in ~/.config/hidra/config.toml.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::catalog::CONTROLS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub dpi_stages: Vec<u16>,
    pub default_stage: usize,
    /// Control id → action id.
    pub buttons: BTreeMap<String, String>,
    /// Control id → shell command, for controls set to "cmd".
    pub commands: BTreeMap<String, String>,
    /// Control id → shortcut text, for controls set to "key".
    pub shortcuts: BTreeMap<String, String>,
    pub slot_names: Vec<String>,
    pub app_profiles: Vec<AppProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppProfile {
    pub name: String,
    pub wm_class: String,
    pub enabled: bool,
    pub overrides: Vec<Override>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Override {
    pub control: String,
    pub action: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dpi_stages: vec![400, 800, 1600, 3200, 6400],
            default_stage: 2,
            buttons: default_buttons(),
            commands: BTreeMap::new(),
            shortcuts: BTreeMap::new(),
            slot_names: ["Desktop", "FPS", "MMO", "Creative", "Spare"].map(String::from).to_vec(),
            app_profiles: vec![
                AppProfile {
                    name: "Firefox".into(),
                    wm_class: "firefox".into(),
                    enabled: true,
                    overrides: vec![
                        Override { control: "tiltl".into(), action: "prevtab".into() },
                        Override { control: "tiltr".into(), action: "nexttab".into() },
                    ],
                },
                AppProfile {
                    name: "Blender".into(),
                    wm_class: "blender".into(),
                    enabled: true,
                    overrides: vec![Override { control: "g5".into(), action: "key".into() }],
                },
            ],
        }
    }
}

pub fn default_buttons() -> BTreeMap<String, String> {
    CONTROLS.iter().map(|c| (c.id.to_string(), c.default_action.to_string())).collect()
}

/// ~/.config/hidra (or $XDG_CONFIG_HOME/hidra).
pub fn dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("hidra"))
}

fn path() -> Option<PathBuf> {
    Some(dir()?.join("config.toml"))
}

impl Config {
    pub fn load() -> Self {
        let Some(p) = path() else { return Self::default() };
        let mut cfg: Config = fs::read_to_string(&p).ok().and_then(|s| toml::from_str(&s).ok()).unwrap_or_default();
        for c in CONTROLS {
            cfg.buttons.entry(c.id.to_string()).or_insert_with(|| c.default_action.to_string());
        }
        cfg.dpi_stages.resize(5, 1600);
        cfg.default_stage = cfg.default_stage.min(4);
        cfg
    }

    pub fn save(&self) -> Result<(), String> {
        let p = path().ok_or("no home directory")?;
        if let Some(dir) = p.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let text = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        let tmp = p.with_extension("toml.tmp");
        fs::write(&tmp, text).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &p).map_err(|e| e.to_string())
    }
}
