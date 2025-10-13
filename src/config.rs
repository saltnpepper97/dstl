use std::path::PathBuf;
use eyre::Result;
use ratatui::style::Color;
use rune_cfg::RuneConfig;
use serde::{Deserialize, Serialize};
use ratatui::widgets::BorderType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SearchPosition {
    Top,
    Bottom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StartMode {
    Single,
    Dual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherTheme {
    pub border: String,
    pub focus: String,
    pub highlight: String,
    pub border_style: String,
    pub highlight_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub dmenu: bool,
    pub search_position: SearchPosition,
    pub start_mode: StartMode,
    pub focus_search_on_switch: bool,
    pub colors: LauncherTheme,
    pub terminal: String,
    pub timeout: u64,
}

impl LauncherTheme {
    /// Convert string from config into a `ratatui::Color`
    pub fn parse_color(color: &str) -> Color {
        match color.to_lowercase().as_str() {
            "black" => Color::Black,
            "red" => Color::Red,
            "green" => Color::Green,
            "yellow" => Color::Yellow,
            "blue" => Color::Blue,
            "magenta" => Color::Magenta,
            "cyan" => Color::Cyan,
            "white" => Color::White,
            // fallback to default
            _ => Color::Reset,
        }
    }

    pub fn parse_border_type(style: &str) -> BorderType {
        match style.to_lowercase().as_str() {
            "plain" => BorderType::Plain,
            "rounded" => BorderType::Rounded,
            "thick" => BorderType::Thick,
            "double" => BorderType::Double,
            _ => BorderType::Plain,
        }
    }
}

fn try_get_bool(config: &RuneConfig, base_path: &str, default: bool) -> bool {
    let hyphenated = base_path.replace('_', "-");
    if let Ok(val) = config.get::<bool>(&hyphenated) {
        return val;
    }
    let underscored = base_path.replace('-', "_");
    if let Ok(val) = config.get::<bool>(&underscored) {
        return val;
    }
    default
}

fn try_get_string(config: &RuneConfig, base_path: &str, default: &str) -> String {
    let hyphenated = base_path.replace('_', "-");
    if let Ok(val) = config.get::<String>(&hyphenated) {
        return val;
    }
    let underscored = base_path.replace('-', "_");
    if let Ok(val) = config.get::<String>(&underscored) {
        return val;
    }
    default.to_string()
}

fn try_get_u64(config: &RuneConfig, base_path: &str, default: u64) -> u64 {
    let hyphenated = base_path.replace('_', "-");
    if let Ok(val) = config.get::<u64>(&hyphenated) {
        return val;
    }
    let underscored = base_path.replace('-', "_");
    if let Ok(val) = config.get::<u64>(&underscored) {
        return val;
    }
    default
}

// --- Load config ---
pub fn load_config(path: &str) -> Result<LauncherConfig> {
    let content = std::fs::read_to_string(path)?;
    let config = RuneConfig::from_str(&content)?;
    
    let dmenu = try_get_bool(&config, "launcher.dmenu", false);
    let terminal = try_get_string(&config, "launcher.terminal", "foot");
    let timeout = try_get_u64(&config, "launcher.timeout", 100);

    let search_position_str = try_get_string(&config, "launcher.search_position", "top");
    let search_position = match search_position_str.to_lowercase().as_str() {
        "bottom" => SearchPosition::Bottom,
        _ => SearchPosition::Top,
    };
    
    let start_mode_str = try_get_string(&config, "launcher.startup_mode", "single");
    let start_mode = match start_mode_str.to_lowercase().as_str() {
        "dual" => StartMode::Dual,
        _ => StartMode::Single,
    };
    
    let border_color = try_get_string(&config, "launcher.theme.border", "White");
    let focus_color = try_get_string(&config, "launcher.theme.focus", "Yellow");
    let highlight_color = try_get_string(&config, "launcher.theme.highlight", "Blue");
    let border_style = try_get_string(&config, "launcher.theme.border_style", "plain");
    let highlight_type = try_get_string(&config, "launcher.theme.highlight_type", "background");

    let colors = LauncherTheme {
        border: border_color,
        focus: focus_color,
        highlight: highlight_color,
        border_style,
        highlight_type,
    };
    
    let focus_search = try_get_bool(&config, "launcher.focus_search_on_switch", false);
    
    Ok(LauncherConfig { 
        dmenu, 
        search_position, 
        start_mode,
        focus_search_on_switch: focus_search,
        colors,
        terminal,
        timeout,
    })
}

pub fn load_launcher_config() -> LauncherConfig {
    let path = find_config().expect("No launcher.rune config found");
    load_config(&path.to_string_lossy()).expect("Failed to parse launcher.rune config")
}

fn find_config() -> Option<PathBuf> {
    if let Some(home) = dirs::config_dir() {
        let user_config = home.join("dstl").join("dstl.rune");
        if user_config.exists() {
            return Some(user_config);
        }
    }
    let default_config = PathBuf::from("/usr/share/doc/dstl/dstl.rune");
    if default_config.exists() {
        return Some(default_config);
    }
    None
}
