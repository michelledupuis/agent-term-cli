use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const DEFAULT_SHELL: &str = "bash";
const DEFAULT_COLS: u16 = 120;
const DEFAULT_ROWS: u16 = 40;
const DEFAULT_POLL_MS: u64 = 100;
const DEFAULT_SCROLLBACK_LINES: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_shell")]
    pub default_shell: String,
    #[serde(default = "default_cols")]
    pub default_cols: u16,
    #[serde(default = "default_rows")]
    pub default_rows: u16,
    #[serde(default = "default_poll_ms")]
    pub poll_interval_ms: u64,
    #[serde(default = "default_scrollback")]
    pub scrollback_lines: usize,
}

fn default_shell() -> String {
    if cfg!(windows) {
        "cmd".to_string()
    } else {
        DEFAULT_SHELL.to_string()
    }
}
fn default_cols() -> u16 {
    DEFAULT_COLS
}
fn default_rows() -> u16 {
    DEFAULT_ROWS
}
fn default_poll_ms() -> u64 {
    DEFAULT_POLL_MS
}
fn default_scrollback() -> usize {
    DEFAULT_SCROLLBACK_LINES
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_shell: default_shell(),
            default_cols: default_cols(),
            default_rows: default_rows(),
            poll_interval_ms: default_poll_ms(),
            scrollback_lines: default_scrollback(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    pub name: String,
    pub url: String,
    pub token: String,
    #[serde(default)]
    pub shell: Option<String>,
    #[serde(default)]
    pub cols: Option<u16>,
    #[serde(default)]
    pub rows: Option<u16>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub terminals: Vec<TerminalConfig>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;
        let config: Config =
            toml::from_str(&content).with_context(|| format!("Failed to parse config: {}", path.display()))?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn default_path() -> PathBuf {
        if cfg!(windows) {
            let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(appdata).join("agent-term-cli").join("config.toml")
        } else if cfg!(target_os = "macos") {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home)
                .join("Library/Application Support/agent-term-cli/config.toml")
        } else {
            let config_dir = std::env::var("XDG_CONFIG_HOME")
                .unwrap_or_else(|_| {
                    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                    format!("{home}/.config")
                });
            PathBuf::from(config_dir).join("agent-term-cli").join("config.toml")
        }
    }

    pub fn history_path(name: &str) -> PathBuf {
        // Sanitize name for use as filename
        let safe_name: String = name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        if cfg!(windows) {
            let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(appdata).join("agent-term-cli").join("history").join(format!("{safe_name}.json"))
        } else if cfg!(target_os = "macos") {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home)
                .join("Library/Application Support/agent-term-cli/history")
                .join(format!("{safe_name}.json"))
        } else {
            let config_dir = std::env::var("XDG_CONFIG_HOME")
                .unwrap_or_else(|_| {
                    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                    format!("{home}/.config")
                });
            PathBuf::from(config_dir)
                .join("agent-term-cli/history")
                .join(format!("{safe_name}.json"))
        }
    }
}
