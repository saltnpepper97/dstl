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
    /// Returns true if this app should be run in a terminal.
    pub fn needs_terminal(&self) -> bool {
        // Check for obvious CLI category or "terminal" hints
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

        Self {
            mode,
            single_pane_mode,
            should_quit: false,
            search_query: String::new(),
            categories,
            apps,
            selected_category: 0,
            selected_app: 0,
            focus,
            app_to_launch: None,
            config: config.clone(),
            fuzzy_matcher: SkimMatcherV2::default(),
        }
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

    /// Check if an app matches the search query using fuzzy matching
    pub fn matches_search(&self, app_name: &str, query: &str) -> Option<i64> {
        if query.is_empty() {
            return Some(0); // Empty query matches everything
        }
        self.fuzzy_matcher.fuzzy_match(app_name, query)
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
        let mut seen_apps: HashSet<String> = HashSet::new(); // Track seen app names

        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/home"));
        let local_dir = format!("{}/.local/share/applications", home);

        let paths = vec![local_dir, "/usr/share/applications".to_string()];

        for dir in paths {
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) != Some("desktop") {
                        continue;
                    }

                    if let Ok(content) = fs::read_to_string(&path) {
                        let mut name = None;
                        let mut generic_name = None;
                        let mut exec = None;
                        let mut categories = None;
                        let mut no_display = false;
                        let mut in_desktop_entry = false;
                        let mut terminal = false;

                        for line in content.lines() {
                            let line = line.trim();
                            
                            // Track if we're in the [Desktop Entry] section
                            if line == "[Desktop Entry]" {
                                in_desktop_entry = true;
                                continue;
                            } else if line.starts_with('[') {
                                in_desktop_entry = false;
                                continue;
                            }
                            
                            if !in_desktop_entry {
                                continue;
                            }
                            
                            // Get the base Name= (without locale)
                            if line.starts_with("Name=") && !line.contains('[') {
                                name = Some(line["Name=".len()..].trim().to_string());
                            }
                            // Fallback to GenericName if Name is not found
                            if line.starts_with("GenericName=") && !line.contains('[') {
                                generic_name = Some(line["GenericName=".len()..].trim().to_string());
                            }
                            if line.starts_with("Exec=") {
                                exec = Some(line["Exec=".len()..].trim().to_string());
                            }
                            if line.starts_with("Categories=") {
                                categories = Some(line["Categories=".len()..].trim().to_string());
                            }
                            if line == "NoDisplay=true" || line == "Hidden=true" {
                                no_display = true;
                            }
                            if line == "Terminal=true" {
                                terminal = true;
                            }
                        }

                        // Skip apps marked as NoDisplay or Hidden
                        if no_display {
                            continue;
                        }
                        
                        // Use Name, or fallback to GenericName
                        let name = name.or(generic_name);

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
                                // Apps without Categories field go to Utilities by default
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

        // Build the list of grouped categories in a fixed order
        let category_order = vec![
            "Utilities", "Development", "Network", "Audio/Video", "Graphics",
            "System", "Office", "Games", "Education", "Settings"
        ];
        let categories: Vec<String> = category_order
            .into_iter()
            .filter(|c| category_map.contains_key(*c))
            .map(|s| s.to_string())
            .collect();

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
