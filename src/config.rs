use std::path::{Path, PathBuf};
use std::process;
use eyre::{Result, eyre};
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

/// Manually parse and load gather imports since rune_cfg doesn't do this automatically
fn load_config_with_gather(path: &Path) -> Result<RuneConfig> {
    use std::fs;
    use regex::Regex;

    // Read the main config file
    let content = fs::read_to_string(path)
        .map_err(|e| eyre!("Failed to read config file: {}", e))?;

    // Parse gather statements manually, but only from non-commented lines
    // Format: gather "path/to/file.rune" [as alias]
    let gather_regex = Regex::new(r#"gather\s+"([^"]+)"(?:\s+as\s+(\w+))?"#).unwrap();
    
    // Create the main config
    let mut config = RuneConfig::from_str(&content)
        .map_err(|e| eyre!("Failed to parse main config: {}", e))?;

    // Process each line to find gather statements (excluding comments)
    for line in content.lines() {
        let trimmed = line.trim();
        
        // Skip commented lines
        if trimmed.starts_with('#') {
            continue;
        }
        
        // Find gather statement in this line
        if let Some(caps) = gather_regex.captures(line) {
            let gather_path_str = &caps[1];
            let alias = caps.get(2).map(|m| m.as_str().to_string());

            // Expand tilde if present
            let expanded_path = if gather_path_str.starts_with("~/") {
                dirs::home_dir()
                    .ok_or_else(|| eyre!("Could not determine home directory"))?
                    .join(&gather_path_str[2..])
            } else {
                // If relative path, make it relative to config directory
                let config_dir = path.parent().unwrap_or_else(|| Path::new("."));
                config_dir.join(gather_path_str)
            };

            // Check if file exists
            if !expanded_path.exists() {
                continue;
            }

            // Load the gathered file
            let gather_content = fs::read_to_string(&expanded_path)
                .map_err(|e| eyre!("Failed to read gather file {:?}: {}", expanded_path, e))?;

            let gather_config = RuneConfig::from_str(&gather_content)
                .map_err(|e| eyre!("Failed to parse gather file {:?}: {}", expanded_path, e))?;

            // Get the document from the gathered config
            if let Some(doc) = gather_config.document() {
                let import_alias = alias.unwrap_or_else(|| {
                    // Use filename without extension as default alias
                    expanded_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("imported")
                        .to_string()
                });

                config.inject_import(import_alias, doc.clone());
            }
        }
    }

    Ok(config)
}

/// Load config with gather support and theme priority system
pub fn load_config(path: &str) -> Result<LauncherConfig> {
    let path_buf = PathBuf::from(path);
    let config = load_config_with_gather(&path_buf)?;

    // --- Fetch values with validation ---
    let dmenu = get_config_or(&config, "launcher.dmenu", false);
    let terminal = get_config_or(&config, "launcher.terminal", "foot".to_string());
    let timeout = get_config_or(&config, "launcher.timeout", 0u64);
    let max_recent_apps: usize = get_config_or(&config, "launcher.max_recent_apps", 15u64) as usize;
    let recent_first = get_config_or(&config, "launcher.recent_first", false);

    // Validate search_position
    let search_position_str: String = get_config_or(&config, "launcher.search_position", "top".to_string());
    let search_position = match search_position_str.to_lowercase().as_str() {
        "top" => SearchPosition::Top,
        "bottom" => SearchPosition::Bottom,
        _ => SearchPosition::Top,
    };

    // Validate startup_mode
    let start_mode_str: String = get_config_or(&config, "launcher.startup_mode", "single".to_string());
    let start_mode = match start_mode_str.to_lowercase().as_str() {
        "single" => StartMode::Single,
        "dual" => StartMode::Dual,
        _ => StartMode::Single,
    };

    // Load colors with theme priority system
    let (border_color, focus_color, highlight_color, cursor_color) = load_theme_colors(&config)?;

    let cursor_shape_str: String = get_config_or(&config, "launcher.theme.cursor_shape", "block".to_string());
    let cursor_shape = match cursor_shape_str.to_lowercase().as_str() {
        "block" => CursorShape::Block,
        "underline" => CursorShape::Underline,
        "pipe" => CursorShape::Pipe,
        _ => CursorShape::Block,
    };

    let cursor_blink_interval: u64 = get_config_or(&config, "launcher.theme.cursor_blink_interval", 0u64);
    let border_style: String = get_config_or(&config, "launcher.theme.border_style", "plain".to_string());
    let highlight_type: String = get_config_or(&config, "launcher.theme.highlight_type", "background".to_string());
    let focus_search: bool = get_config_or(&config, "launcher.focus_search_on_switch", true);

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
            let test_path = format!("{}.launcher.theme.border", alias);
            if let Ok(val) = config.get::<String>(&test_path) {
                border = Some(val);
                focus = config.get::<String>(&format!("{}.launcher.theme.focus", alias)).ok();
                highlight = config.get::<String>(&format!("{}.launcher.theme.highlight", alias)).ok();
                cursor = config.get::<String>(&format!("{}.launcher.theme.cursor_color", alias)).ok();
                break;
            }
        }
    }

    // PRIORITY 2: Check for top-level theme (from non-aliased gather or main config)
    if border.is_none() {
        border = config.get::<String>("launcher.theme.border").ok();
        focus = config.get::<String>("launcher.theme.focus").ok();
        highlight = config.get::<String>("launcher.theme.highlight").ok();
        cursor = config.get::<String>("launcher.theme.cursor_color").ok();
    }

    // PRIORITY 3: Check for "theme" document
    if border.is_none() && config.has_document("theme") {
        border = config.get::<String>("theme.launcher.theme.border").ok();
        focus = config.get::<String>("theme.launcher.theme.focus").ok();
        highlight = config.get::<String>("theme.launcher.theme.highlight").ok();
        cursor = config.get::<String>("theme.launcher.theme.cursor_color").ok();
    }

    // Defaults
    let border = border.unwrap_or_else(|| "#ffffff".to_string());
    let focus = focus.unwrap_or_else(|| "#00ff00".to_string());
    let highlight = highlight.unwrap_or_else(|| "#0000ff".to_string());
    let cursor = cursor.unwrap_or_else(|| focus.clone());

    Ok((border, focus, highlight, cursor))
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
