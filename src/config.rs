use std::path::PathBuf;
use std::process;
use eyre::Result;
use ratatui::style::Color;
use ratatui::widgets::BorderType;
use rune_cfg::{RuneConfig, Value, RuneError};
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CursorShape {
    Block,      // █
    Underline,  // _
    Pipe,       // |
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherTheme {
    pub border: String,
    pub focus: String,
    pub highlight: String,
    pub border_style: String,
    pub highlight_type: String,
    pub cursor_color: String,
    pub cursor_shape: CursorShape,
    pub cursor_blink_interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DstlConfig {
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
    /// Convert hex string to ratatui::Color
    pub fn parse_color(color: &str) -> Color {
        let color = color.trim();
        
        // Handle hex colors (#RGB, #RRGGBB, #RRGGBBAA)
        if color.starts_with('#') {
            let hex = &color[1..];
            
            match hex.len() {
                // #RGB format
                3 => {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        u8::from_str_radix(&hex[0..1], 16),
                        u8::from_str_radix(&hex[1..2], 16),
                        u8::from_str_radix(&hex[2..3], 16),
                    ) {
                        // Expand single digit to double (e.g., F -> FF)
                        return Color::Rgb(r * 17, g * 17, b * 17);
                    }
                }
                // #RRGGBB format
                6 => {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        u8::from_str_radix(&hex[0..2], 16),
                        u8::from_str_radix(&hex[2..4], 16),
                        u8::from_str_radix(&hex[4..6], 16),
                    ) {
                        return Color::Rgb(r, g, b);
                    }
                }
                // #RRGGBBAA format (ignore alpha for now)
                8 => {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        u8::from_str_radix(&hex[0..2], 16),
                        u8::from_str_radix(&hex[2..4], 16),
                        u8::from_str_radix(&hex[4..6], 16),
                    ) {
                        return Color::Rgb(r, g, b);
                    }
                }
                _ => {}
            }
        }
        
        // Fallback to reset if parsing fails
        Color::Reset
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

/// Helper: tries key as-is, then _ → -, then - → _
fn get_config_or<T>(
    config: &RuneConfig,
    key: &str,
    default: T,
) -> T
where
    T: Clone + TryFrom<Value, Error = RuneError>,
{
    let variants = [
        key.to_string(),
        key.replace('_', "-"),
        key.replace('-', "_"),
    ];

    for k in variants {
        if let Ok(val) = config.get::<T>(&k) {
            return val;
        }
    }

    default
}

/// Extract DstlConfig from a loaded RuneConfig
fn extract_dstl_config(config: RuneConfig) -> Result<DstlConfig> {
    // --- Fetch values with validation ---
    let dmenu = get_config_or(&config, "dstl.dmenu", false);
    let terminal = get_config_or(&config, "dstl.terminal", "foot".to_string());
    let timeout = get_config_or(&config, "dstl.timeout", 0u64);
    let max_recent_apps: usize = get_config_or(&config, "dstl.max_recent_apps", 15u64) as usize;
    let recent_first = get_config_or(&config, "dstl.recent_first", false);

    // Validate search_position
    let search_position_str: String = get_config_or(&config, "dstl.search_position", "top".to_string());
    let search_position = match search_position_str.to_lowercase().as_str() {
        "top" => SearchPosition::Top,
        "bottom" => SearchPosition::Bottom,
        _ => SearchPosition::Top,
    };

    // Validate startup_mode
    let start_mode_str: String = get_config_or(&config, "dstl.startup_mode", "single".to_string());
    let start_mode = match start_mode_str.to_lowercase().as_str() {
        "single" => StartMode::Single,
        "dual" => StartMode::Dual,
        _ => StartMode::Single,
    };

    // Load colors with theme priority system
    let (border_color, focus_color, highlight_color, cursor_color) = load_theme_colors(&config)?;

    let cursor_shape_str: String = get_config_or(&config, "dstl.theme.cursor_shape", "block".to_string());
    let cursor_shape = match cursor_shape_str.to_lowercase().as_str() {
        "block" => CursorShape::Block,
        "underline" => CursorShape::Underline,
        "pipe" => CursorShape::Pipe,
        _ => CursorShape::Block,
    };

    let cursor_blink_interval: u64 = get_config_or(&config, "dstl.theme.cursor_blink_interval", 0u64);
    let border_style: String = get_config_or(&config, "dstl.theme.border_style", "plain".to_string());
    let highlight_type: String = get_config_or(&config, "dstl.theme.highlight_type", "background".to_string());
    let focus_search: bool = get_config_or(&config, "dstl.focus_search_on_switch", true);

    let colors = LauncherTheme {
        border: border_color,
        focus: focus_color,
        highlight: highlight_color,
        border_style,
        highlight_type,
        cursor_color,
        cursor_shape,
        cursor_blink_interval,
    };

    Ok(DstlConfig {
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

/// Load theme colors with priority system similar to claw
fn load_theme_colors(config: &RuneConfig) -> Result<(String, String, String, String)> {
    let mut border = None;
    let mut focus = None;
    let mut highlight = None;
    let mut cursor = None;

    // PRIORITY 1: Check for aliased gather imports
    let aliases = config.import_aliases();
    for alias in aliases {
        if config.has_document(&alias) {
            // Test if this import has theme data
            let test_path = format!("{}.dstl.theme.border", alias);
            if let Ok(val) = config.get::<String>(&test_path) {
                border = Some(val);
                focus = config.get::<String>(&format!("{}.dstl.theme.focus", alias)).ok();
                highlight = config.get::<String>(&format!("{}.dstl.theme.highlight", alias)).ok();
                cursor = config.get::<String>(&format!("{}.dstl.theme.cursor_color", alias)).ok();
                break;
            }
        }
    }

    // PRIORITY 2: Check for top-level theme (from non-aliased gather or main config)
    if border.is_none() {
        border = config.get::<String>("dstl.theme.border").ok();
        focus = config.get::<String>("dstl.theme.focus").ok();
        highlight = config.get::<String>("dstl.theme.highlight").ok();
        cursor = config.get::<String>("dstl.theme.cursor_color").ok();
    }

    // PRIORITY 3: Check for "theme" document
    if border.is_none() && config.has_document("theme") {
        border = config.get::<String>("theme.dstl.theme.border").ok();
        focus = config.get::<String>("theme.dstl.theme.focus").ok();
        highlight = config.get::<String>("theme.dstl.theme.highlight").ok();
        cursor = config.get::<String>("theme.dstl.theme.cursor_color").ok();
    }

    // Defaults
    let border = border.unwrap_or_else(|| "#ffffff".to_string());
    let focus = focus.unwrap_or_else(|| "#00ff00".to_string());
    let highlight = highlight.unwrap_or_else(|| "#0000ff".to_string());
    let cursor = cursor.unwrap_or_else(|| focus.clone());

    Ok((border, focus, highlight, cursor))
}

/// Top-level config loader that exits gracefully on failure.
pub fn load_launcher_config() -> DstlConfig {
    let user_config = dirs::config_dir()
        .map(|c| c.join("dstl/dstl.rune"))
        .unwrap_or_else(|| PathBuf::from("~/.config/dstl/dstl.rune"));
    
    let system_config = PathBuf::from("/usr/share/doc/dstl/dstl.rune");
    
    // Load config with automatic import resolution and fallback support
    let config = RuneConfig::from_file_with_fallback(&user_config, &system_config)
        .unwrap_or_else(|e| {
            eprintln!("❌ Configuration error:\n{}", e);
            process::exit(1);
        });

    // Extract DstlConfig from the loaded RuneConfig
    extract_dstl_config(config).unwrap_or_else(|e| {
        eprintln!("❌ Configuration parsing error:\n{}", e);
        process::exit(1);
    })
}
