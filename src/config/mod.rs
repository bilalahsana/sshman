use directories::ProjectDirs;
use std::path::PathBuf;

pub fn config_dir() -> Option<PathBuf> {
    ProjectDirs::from("com", "sshman", "sshman").map(|dirs| dirs.config_dir().to_path_buf())
}

pub fn data_dir() -> Option<PathBuf> {
    ProjectDirs::from("com", "sshman", "sshman").map(|dirs| dirs.data_dir().to_path_buf())
}

pub fn hosts_file() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("hosts.json"))
}

pub fn config_file() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("config.toml"))
}

pub fn ensure_dirs() -> anyhow::Result<()> {
    if let Some(dir) = config_dir() {
        std::fs::create_dir_all(dir)?;
    }
    if let Some(dir) = data_dir() {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}

pub mod settings {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
    pub enum Theme {
        #[default]
        Dark,
        Light,
        Nord,
        Gruvbox,
        Catppuccin,
    }

    impl Theme {
        pub fn all() -> &'static [Theme] {
            &[
                Theme::Dark,
                Theme::Light,
                Theme::Nord,
                Theme::Gruvbox,
                Theme::Catppuccin,
            ]
        }

        pub fn name(&self) -> &'static str {
            match self {
                Theme::Dark => "dark",
                Theme::Light => "light",
                Theme::Nord => "nord",
                Theme::Gruvbox => "gruvbox",
                Theme::Catppuccin => "catppuccin",
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct ThemeColors {
        pub bg_primary: String,
        pub bg_secondary: String,
        pub fg_primary: String,
        pub fg_secondary: String,
        pub accent: String,
        pub highlight: String,
        pub error: String,
        pub success: String,
        pub border: String,
    }

    impl ThemeColors {
        pub fn dark() -> Self {
            Self {
                bg_primary: "#1e1e1e".to_string(),
                bg_secondary: "#252526".to_string(),
                fg_primary: "#d4d4d4".to_string(),
                fg_secondary: "#808080".to_string(),
                accent: "#00aaff".to_string(),
                highlight: "#264f78".to_string(),
                error: "#f44747".to_string(),
                success: "#89d185".to_string(),
                border: "#3c3c3c".to_string(),
            }
        }

        pub fn light() -> Self {
            Self {
                bg_primary: "#ffffff".to_string(),
                bg_secondary: "#f3f3f3".to_string(),
                fg_primary: "#333333".to_string(),
                fg_secondary: "#6e6e6e".to_string(),
                accent: "#0078d4".to_string(),
                highlight: "#add6ff".to_string(),
                error: "#d32f2f".to_string(),
                success: "#388e3c".to_string(),
                border: "#cccccc".to_string(),
            }
        }

        pub fn nord() -> Self {
            Self {
                bg_primary: "#2e3440".to_string(),
                bg_secondary: "#3b4252".to_string(),
                fg_primary: "#eceff4".to_string(),
                fg_secondary: "#d8dee9".to_string(),
                accent: "#88c0d0".to_string(),
                highlight: "#4c566a".to_string(),
                error: "#bf616a".to_string(),
                success: "#a3be8c".to_string(),
                border: "#4c566a".to_string(),
            }
        }

        pub fn gruvbox() -> Self {
            Self {
                bg_primary: "#282828".to_string(),
                bg_secondary: "#3c3836".to_string(),
                fg_primary: "#ebdbb2".to_string(),
                fg_secondary: "#a59da7".to_string(),
                accent: "#fabd2f".to_string(),
                highlight: "#504945".to_string(),
                error: "#fb4934".to_string(),
                success: "#b8bb26".to_string(),
                border: "#504945".to_string(),
            }
        }

        pub fn catppuccin() -> Self {
            Self {
                bg_primary: "#1e1e2e".to_string(),
                bg_secondary: "#313244".to_string(),
                fg_primary: "#cdd6f4".to_string(),
                fg_secondary: "#a6adc8".to_string(),
                accent: "#89b4fa".to_string(),
                highlight: "#45475a".to_string(),
                error: "#f38ba8".to_string(),
                success: "#a6e3a1".to_string(),
                border: "#45475a".to_string(),
            }
        }

        pub fn from_theme(theme: Theme) -> Self {
            match theme {
                Theme::Dark => Self::dark(),
                Theme::Light => Self::light(),
                Theme::Nord => Self::nord(),
                Theme::Gruvbox => Self::gruvbox(),
                Theme::Catppuccin => Self::catppuccin(),
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct KeyBindings {
        pub quit: String,
        pub up: String,
        pub down: String,
        pub left: String,
        pub right: String,
        pub search: String,
        pub connect: String,
        pub add: String,
        pub edit: String,
        pub delete: String,
        pub toggle_favorite: String,
        pub copy_hostname: String,
        pub copy_command: String,
        pub ping: String,
        pub ssh_copy_id: String,
        pub help: String,
        pub command_palette: String,
        pub sort: String,
        pub theme: String,
        pub top: String,
        pub bottom: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct AppConfig {
        pub theme: Theme,
        pub keybindings: KeyBindings,
        pub show_status_bar: bool,
        pub show_groups: bool,
        pub auto_refresh: bool,
        pub ping_timeout_ms: u64,
        pub startup_view: String,
        pub show_connections: bool,
    }

    impl AppConfig {
        pub fn load() -> anyhow::Result<Self> {
            if let Some(path) = super::config_file() {
                if path.exists() {
                    let content = std::fs::read_to_string(&path)?;
                    return Ok(toml::from_str(&content)?);
                }
            }
            Ok(Self::default())
        }

        pub fn save(&self) -> anyhow::Result<()> {
            if let Some(path) = super::config_file() {
                super::ensure_dirs()?;
                let content = toml::to_string_pretty(self)?;
                std::fs::write(&path, content)?;
            }
            Ok(())
        }
    }
}
