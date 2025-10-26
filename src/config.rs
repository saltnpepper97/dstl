use std::path::PathBuf;
use std::process;
use eyre::{Result, eyre};
use ratatui::style::Color;
use ratatui::widgets::BorderType;
use rune_cfg::RuneConfig;
use serde::{Deserialize, Serialize};

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
    pub max_recent_apps: usize,
    pub recent_first: bool,
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

// --- Load config ---
pub fn load_config(path: &str) -> Result<LauncherConfig> {
    let config = RuneConfig::from_file(path)
        .map_err(|e| eyre!("Failed to load config: {}", e))?;

    // --- Fetch values with validation ---
    let dmenu = config.get_or("launcher.dmenu", false);
    let terminal = config.get_or("launcher.terminal", "foot".to_string());
    let timeout = config.get("launcher.timeout")
        .map_err(|e| eyre!("{}", e))?;
    
    // Try both underscore and hyphen versions for max_recent_apps
    let max_recent_apps: usize = config.get("launcher.max_recent_apps")
        .or_else(|_| config.get("launcher.max-recent-apps"))
        .unwrap_or(15u64) as usize;
    
    // Try both underscore and hyphen versions for recent_first
    let recent_first = config.get("launcher.recent_first")
        .or_else(|_| config.get("launcher.recent-first"))
        .unwrap_or(false);

    // Validate search_position
    let search_position_str = config.get_string_enum(
        "launcher.search_position", 
        &["top", "bottom"]
    ).unwrap_or_else(|_| "top".to_string());
    
    let search_position = match search_position_str.to_lowercase().as_str() {
        "top" => SearchPosition::Top,
        "bottom" => SearchPosition::Bottom,
        _ => SearchPosition::Top, // Already validated, this is just a fallback
    };

    // Validate startup_mode
    let start_mode_str = config.get_string_enum(
        "launcher.startup_mode",
        &["single", "dual"]
    ).unwrap_or_else(|_| "single".to_string());
    
    let start_mode = match start_mode_str.to_lowercase().as_str() {
        "single" => StartMode::Single,
        "dual" => StartMode::Dual,
        _ => StartMode::Single,
    };

    // Validate colors
    let border_color = config.get_string_enum(
        "launcher.theme.border",
        &["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white", "reset"]
    ).unwrap_or_else(|_| "white".to_string());

    let focus_color = config.get_string_enum(
        "launcher.theme.focus",
        &["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white", "reset"]
    ).map_err(|e| eyre!("{}", e))?;

    let highlight_color = config.get_string_enum(
        "launcher.theme.highlight",
        &["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white", "reset"]
    ).unwrap_or_else(|_| "blue".to_string());

    // Validate border style
    let border_style = config.get_string_enum(
        "launcher.theme.border_style",
        &["plain", "rounded", "thick", "double"]
    ).map_err(|e| eyre!("{}", e))?;

    let highlight_type = config.get_or("launcher.theme.highlight_type", "background".to_string());
    
    // Validate boolean - will automatically error if value is invalid
    let focus_search = config.get("launcher.focus_search_on_switch")
        .map_err(|e| eyre!("{}", e))?;

    let colors = LauncherTheme {
        border: border_color,
        focus: focus_color,
        highlight: highlight_color,
        border_style,
        highlight_type,
    };

    Ok(LauncherConfig {
        dmenu,
        search_position,
        start_mode,
        focus_search_on_switch: focus_search,
        colors,
        terminal,
        timeout,
        max_recent_apps,
        recent_first,
    })
}

/// Top-level config loader that exits gracefully on failure.
pub fn load_launcher_config() -> LauncherConfig {
    let path = find_config().expect("No launcher.rune config found");
    match load_config(&path.to_string_lossy()) {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("❌ Configuration error:\n{}", err);
            process::exit(1);
        }
    }
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
