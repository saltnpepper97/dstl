use crate::ui::layout;
use crate::app::{App, Focus};
use crate::config::{LauncherConfig, SearchPosition};
use ratatui::Frame;

pub fn draw(
    f: &mut Frame,
    app: &App,
    search_query: &str,
    apps: &[String],
    selected: usize,
    focus: Focus,
    search_position: SearchPosition,
    config: &LauncherConfig,
) {
    let chunks = layout::vertical_split(f, 3, search_position);
    
    // Filter apps using fuzzy matching and sort by score
    let filtered_apps: Vec<String> = if search_query.is_empty() {
        // If recent_first is enabled, prepend recent apps
        if config.recent_first && !app.recent_apps.is_empty() {
            let mut result = Vec::new();
            
            // Add recent apps first (only those that exist in the apps list)
            for recent in &app.recent_apps {
                if apps.contains(recent) && !result.contains(recent) {
                    result.push(recent.clone());
                }
            }
            
            // Add remaining apps (excluding those already added from recent)
            for app_name in apps {
                if !result.contains(app_name) {
                    result.push(app_name.clone());
                }
            }
            
            result
        } else {
            // Normal alphabetical order
            apps.to_vec()
        }
    } else {
        // When searching, use fuzzy matching and sort by score
        let mut apps_with_scores: Vec<(String, i64)> = apps.iter()
            .filter_map(|a| {
                app.matches_search(a, search_query).map(|score| (a.clone(), score))
            })
            .collect();
        
        // Sort by score (higher is better)
        apps_with_scores.sort_by(|a, b| b.1.cmp(&a.1));
        
        apps_with_scores.into_iter().map(|(name, _)| name).collect()
    };
    
    // Draw apps list — pass config so border color is correct
    layout::render_list(
        f,
        chunks.1,
        " Apps ",
        &filtered_apps,
        selected,
        focus == Focus::Apps,
        config,
    );
    
    // Draw search bar, always using current focus to override color
    layout::render_search_bar(
        f,
        chunks.0,
        search_query,
        focus,
        config,
    );
}
