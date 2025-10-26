use std::fs;
use std::path::Path;
use crate::config::LauncherConfig;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Search,
    Categories,
    Apps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    SinglePane,
    DualPane,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinglePaneMode {
    Dmenu,       // load apps from PATH (dmenu style)
    DesktopApps, // load .desktop apps
}

pub struct App {
    pub mode: Mode,
    pub single_pane_mode: SinglePaneMode,
    pub should_quit: bool,
    pub search_query: String,
    pub categories: Vec<String>,
    pub apps: Vec<AppEntry>,
    pub recent_apps: Vec<String>,
    pub selected_category: usize,
    pub selected_app: usize,
    pub focus: Focus,
    pub app_to_launch: Option<String>,
    pub config: LauncherConfig,
    fuzzy_matcher: SkimMatcherV2,
}

impl Clone for App {
    fn clone(&self) -> Self {
        Self {
            mode: self.mode,
            single_pane_mode: self.single_pane_mode,
            should_quit: self.should_quit,
            search_query: self.search_query.clone(),
            categories: self.categories.clone(),
            apps: self.apps.clone(),
            recent_apps: self.recent_apps.clone(),
            selected_category: self.selected_category,
            selected_app: self.selected_app,
            focus: self.focus,
            app_to_launch: self.app_to_launch.clone(),
            config: self.config.clone(),
            fuzzy_matcher: SkimMatcherV2::default(),
        }
    }
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("mode", &self.mode)
            .field("single_pane_mode", &self.single_pane_mode)
            .field("should_quit", &self.should_quit)
            .field("search_query", &self.search_query)
            .field("categories", &self.categories)
            .field("apps", &self.apps)
            .field("recent_apps", &self.recent_apps)
            .field("selected_category", &self.selected_category)
            .field("selected_app", &self.selected_app)
            .field("focus", &self.focus)
            .field("app_to_launch", &self.app_to_launch)
            .field("config", &self.config)
            .field("fuzzy_matcher", &"SkimMatcherV2")
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct AppEntry {
    pub name: String,
    pub category: String,
    pub exec: String,
    pub terminal: bool,
}

impl AppEntry {
    pub fn needs_terminal(&self) -> bool {
        self.category == "CLI"
            || self.exec.contains("bash")
            || self.exec.contains("sh ")
            || self.exec.contains("python")
            || self.exec.contains("cargo")
            || self.exec.contains("make")
            || self.exec.contains("npm")
    }
}

impl App {
    /// Initialize the app with specified single pane mode and start mode
    pub fn new(single_pane_mode: SinglePaneMode, start_mode: Mode, config: &LauncherConfig) -> Self {
        let (categories, apps, mode, focus) = match start_mode {
            Mode::SinglePane => {
                let (cats, apps) = Self::load_for_mode(single_pane_mode);
                (cats, apps, Mode::SinglePane, Focus::Search)
            }
            Mode::DualPane => {
                let (cats, apps) = Self::load_desktop_apps();
                (cats, apps, Mode::DualPane, Focus::Search)
            }
        };

        let mut app = Self {
            mode,
            single_pane_mode,
            should_quit: false,
            search_query: String::new(),
            categories,
            apps,
            recent_apps: Vec::new(),
            selected_category: 0,
            selected_app: 0,
            focus,
            app_to_launch: None,
            config: config.clone(),
            fuzzy_matcher: SkimMatcherV2::default(),
        };

        // Load recent apps from disk
        let _ = app.load_recent();

        app
    }

    /// Add an app to the recent list
    pub fn add_to_recent(&mut self, app_name: String) {
        // Remove the app if it already exists (to avoid duplicates)
        self.recent_apps.retain(|a| a != &app_name);
        
        // Add to the front of the list
        self.recent_apps.insert(0, app_name);
        
        // Keep only the configured number of recent apps
        let max_recent = self.config.max_recent_apps;
        if self.recent_apps.len() > max_recent {
            self.recent_apps.truncate(max_recent);
        }

        // Save to disk
        let _ = self.save_recent();
    }

    /// Save recent apps to disk
    pub fn save_recent(&self) -> std::io::Result<()> {
        let config_dir = dirs::cache_dir()
            .map(|p| p.join("dstl"))
            .unwrap();
        
        fs::create_dir_all(&config_dir)?;
        let recent_file = config_dir.join("recent.json");
        
        let json = serde_json::to_string_pretty(&self.recent_apps)?;
        fs::write(recent_file, json)?;
        Ok(())
    }

    /// Load recent apps from disk
    pub fn load_recent(&mut self) -> std::io::Result<()> {
        let config_dir = dirs::cache_dir()
            .map(|p| p.join("dstl"))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        
        let recent_file = config_dir.join("recent.json");
        
        if recent_file.exists() {
            let json = fs::read_to_string(recent_file)?;
            self.recent_apps = serde_json::from_str(&json).unwrap_or_default();
        }
        Ok(())
    }

    pub fn visible_apps(&self) -> Vec<&AppEntry> {
        let query = &self.search_query;

        // Start with all apps
        let mut apps: Vec<&AppEntry> = if query.is_empty() {
            self.apps.iter().collect()
        } else {
            // Fuzzy match when searching
            let mut matched: Vec<(&AppEntry, i64)> = self.apps.iter()
                .filter_map(|a| self.matches_search(&a.name, query).map(|score| (a, score)))
                .collect();
            matched.sort_by(|a, b| b.1.cmp(&a.1));
            matched.into_iter().map(|(a, _)| a).collect()
        };

        // If recent_first and not searching, reorder
        if self.search_query.is_empty() && self.config.recent_first && !self.recent_apps.is_empty() {
            let mut recent_list = Vec::new();
            let mut seen = std::collections::HashSet::new();

            // Add recent apps first (must exist in apps)
            for recent_name in &self.recent_apps {
                if let Some(app) = apps.iter().find(|a| a.name == *recent_name) {
                    recent_list.push(*app);
                    seen.insert(recent_name.clone());
                }
            }

            // Add remaining apps
            for app in apps {
                if !seen.contains(&app.name) {
                    recent_list.push(app);
                }
            }

            apps = recent_list;
        }

        apps
    }

    /// Toggle between SinglePane and DualPane
    pub fn toggle_mode(&mut self) {
        match self.mode {
            Mode::SinglePane => {
                let (categories, apps) = Self::load_desktop_apps();
                self.categories = categories;
                self.apps = apps;
                self.mode = Mode::DualPane;
                
                // Keep leftmost pane focused when switching to DualPane
                self.focus = Focus::Categories;
            }
            Mode::DualPane => {
                let (categories, apps) = Self::load_for_mode(self.single_pane_mode);
                self.categories = categories;
                self.apps = apps;
                self.mode = Mode::SinglePane;
                
                // Leftmost pane in SinglePane is Apps
                self.focus = Focus::Apps;
            }
        }

        // Reset selection indexes
        self.selected_category = 0;
        self.selected_app = 0;
    }

    /// Check if an app matches the search query using fuzzy matching (case-insensitive)
    pub fn matches_search(&self, app_name: &str, query: &str) -> Option<i64> {
        if query.is_empty() {
            return Some(0); // Empty query matches everything
        }

        let app_name_lower = app_name.to_lowercase();
        let query_lower = query.to_lowercase();

        // Exact prefix match gets highest priority
        if app_name_lower.starts_with(&query_lower) {
            return Some(i64::MAX); // Push to top
        }

        // Fuzzy match otherwise
        self.fuzzy_matcher.fuzzy_match(&app_name_lower, &query_lower)
    }

    /// Load apps based on the single pane mode
    fn load_for_mode(mode: SinglePaneMode) -> (Vec<String>, Vec<AppEntry>) {
        let (categories, mut apps) = match mode {
            SinglePaneMode::DesktopApps => Self::load_desktop_apps(),
            SinglePaneMode::Dmenu => Self::load_from_path("/usr/bin"),
        };
        
        // Sort apps alphabetically for single pane mode
        apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        
        (categories, apps)
    }

    /// Load .desktop apps from local and system directories
    fn load_desktop_apps() -> (Vec<String>, Vec<AppEntry>) {
        use std::collections::{HashMap, HashSet};

        let mut apps = Vec::new();
        let mut category_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut seen_apps: HashSet<String> = HashSet::new();
        let mut seen_files: HashSet<String> = HashSet::new(); // Track processed .desktop files

        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/home"));
        let local_dir = format!("{}/.local/share/applications", home);

        let paths = vec![local_dir, "/usr/share/applications".to_string()];

        // Get current desktop environment once
        let current_desktops: Vec<String> = std::env::var("XDG_CURRENT_DESKTOP")
            .or_else(|_| std::env::var("DESKTOP_SESSION"))
            .unwrap_or_default()
            .split(':')
            .map(|s| s.trim().to_lowercase())
            .collect();

        for dir in paths {
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) != Some("desktop") {
                        continue;
                    }

                    // Get the base filename to check for duplicates across directories
                    let filename = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();
                    
                    // Skip if we've already processed this .desktop file from another directory
                    if seen_files.contains(&filename) {
                        continue;
                    }
                    seen_files.insert(filename.clone());

                    if let Ok(content) = fs::read_to_string(&path) {
                        let mut name = None;
                        let mut generic_name = None;
                        let mut exec = None;
                        let mut categories = None;
                        let mut no_display = false;
                        let mut terminal = false;
                        let mut only_show_in: Option<Vec<String>> = None;
                        let mut not_show_in: Option<Vec<String>> = None;
                        let mut in_desktop_entry = false;

                        for line in content.lines() {
                            let line = line.trim();
                            
                            // Track sections
                            if line.starts_with('[') {
                                in_desktop_entry = line == "[Desktop Entry]";
                                continue;
                            }
                            
                            // Only parse inside [Desktop Entry] section
                            if !in_desktop_entry {
                                continue;
                            }
                            
                            // Parse key=value pairs
                            if let Some((key, value)) = line.split_once('=') {
                                // Skip localized entries like Name[af]=, Comment[de]=, etc.
                                if key.contains('[') {
                                    continue;
                                }
                                
                                let key = key.trim();
                                let value = value.trim();
                                
                                match key {
                                    "Name" => name = Some(value.to_string()),
                                    "GenericName" => generic_name = Some(value.to_string()),
                                    "Exec" => exec = Some(value.to_string()),
                                    "Categories" => categories = Some(value.to_string()),
                                    "NoDisplay" => no_display = value == "true",
                                    "Hidden" => no_display = no_display || value == "true",
                                    "Terminal" => terminal = value == "true",
                                    "OnlyShowIn" => {
                                        only_show_in = Some(
                                            value.split(';')
                                                .map(|s| s.trim())
                                                .filter(|s| !s.is_empty())
                                                .map(|s| s.to_string())
                                                .collect()
                                        );
                                    }
                                    "NotShowIn" => {
                                        not_show_in = Some(
                                            value.split(';')
                                                .map(|s| s.trim())
                                                .filter(|s| !s.is_empty())
                                                .map(|s| s.to_string())
                                                .collect()
                                        );
                                    }
                                    _ => {}
                                }
                            }
                        }

                        // Skip apps marked as NoDisplay or Hidden
                        if no_display {
                            continue;
                        }
                        
                        // Use Name, or fallback to GenericName
                        let name = name.or(generic_name);

                        // Check OnlyShowIn - skip if specified and current desktop not in list
                        if let Some(desktops) = &only_show_in {
                            let allowed = desktops.iter()
                                .any(|d| current_desktops.contains(&d.to_lowercase()));
                            
                            if !allowed {
                                continue;
                            }
                        }

                        // Check NotShowIn - skip if current desktop is in list
                        if let Some(desktops) = &not_show_in {
                            let blocked = desktops.iter()
                                .any(|d| current_desktops.contains(&d.to_lowercase()));
                            if blocked {
                                continue;
                            }
                        }

                        if let (Some(name), Some(exec)) = (name, exec) {
                            // Skip if we've already seen this app name
                            if seen_apps.contains(&name) {
                                continue;
                            }
                            seen_apps.insert(name.clone());
                            
                            // Determine grouped category             
                            let cat_group = if let Some(cats) = categories {
                                Self::group_category(&cats, &name)
                            } else {
                                Self::group_category("", &name)
                            };

                            // Clean up Exec field codes (%f, %F, %u, %U, etc.)
                            let exec_clean = Self::clean_exec(&exec);

                            apps.push(AppEntry {
                                name: name.clone(),
                                category: cat_group.clone(),
                                exec: exec_clean,
                                terminal,
                            });

                            category_map
                                .entry(cat_group)
                                .or_default()
                                .push(name);
                        }
                    }
                }
            }
        }

        // Build the list of grouped categories with Recent first
        let mut categories = vec!["Recent".to_string()];
        
        let category_order = vec![
            "Utilities", "Development", "Network", "Audio/Video", "Graphics",
            "System", "Office", "Games", "Education", "Settings"
        ];
        categories.extend(
            category_order
                .into_iter()
                .filter(|c| category_map.contains_key(*c))
                .map(|s| s.to_string())
        );

        (categories, apps)
    }

    /// Clean desktop entry Exec field by removing field codes
    fn clean_exec(exec: &str) -> String {
        // Remove field codes like %f, %F, %u, %U, %d, %D, %n, %N, %i, %c, %k, %v, %m
        let mut result = exec.to_string();
        let field_codes = ["%f", "%F", "%u", "%U", "%d", "%D", "%n", "%N", "%i", "%c", "%k", "%v", "%m"];
        for code in &field_codes {
            result = result.replace(code, "");
        }
        result.trim().to_string()
    }

    /// Map raw .desktop categories to simplified groupings 
    fn group_category(raw: &str, app_name: &str) -> String {
        let raw = raw.to_lowercase();

        // Special case for Claw
        if app_name.to_lowercase() == "claw" {
            return "Utilities".to_string();
        }

        if app_name.to_lowercase() == "rofi" {
            return "Utilities".to_string();
        }

        // Prioritize "game" before "network"
        if raw.contains("game") { "Games".to_string() }
        else if raw.contains("utility") { "Utilities".to_string() }
        else if raw.contains("development") { "Development".to_string() }
        else if raw.contains("network") { "Network".to_string() }
        else if raw.contains("audio") || raw.contains("video") { "Audio/Video".to_string() }
        else if raw.contains("graphics") || raw.contains("2dgraphics") || raw.contains("3dgraphics") {
            "Graphics".to_string()
        }
        else if raw.contains("system") { "System".to_string() }
        else if raw.contains("office") { "Office".to_string() }
        else if raw.contains("education") { "Education".to_string() }
        else if raw.contains("settings") { "Settings".to_string() }
        else { "Utilities".to_string() }
    }

    /// Load executables from a directory (dmenu style)
    fn load_from_path<P: AsRef<Path>>(path: P) -> (Vec<String>, Vec<AppEntry>) {
        let mut apps = Vec::new();

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        apps.push(AppEntry {
                            name: name.to_string(),
                            category: "CLI".to_string(),
                            exec: name.to_string(),
                            terminal: true,
                        });
                    }
                }
            }
        }

        // Dmenu-style uses CLI category for consistency
        (vec!["CLI".to_string()], apps)
    }
}
