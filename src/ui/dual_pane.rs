use crate::app::{App, Focus};
use crate::ui::layout;
use crate::config::{LauncherConfig, SearchPosition};
use ratatui::Frame;

/// Draw the dual-pane view
pub fn draw(f: &mut Frame, app: &mut App, search_position: SearchPosition, config: &LauncherConfig) {
    // Split vertically according to search_position
    let (search_area, content_area) = layout::vertical_split(f, 3, search_position);
    
    // --- Search bar ---
    layout::render_search_bar(
        f,
        search_area,
        &app.search_query,
        app.focus,
        config,
    );
    
    // --- Horizontal split with minimum width for categories ---
    let (categories_area, apps_area) = layout::horizontal_split(content_area);
    
    // Get selected category name
    let selected_category_name = app
        .categories
        .get(app.selected_category)
        .cloned()
        .unwrap_or_default();
    
    // Filter apps in selected category with optional search query
    let query = app.search_query.to_lowercase();
    let apps_to_show: Vec<String> = app.apps
        .iter()
        .filter(|a| a.category == selected_category_name &&
                    (app.search_query.is_empty() || a.name.to_lowercase().contains(&query)))
        .map(|a| a.name.clone())
        .collect();
    
    // Clamp selected app index
    if !apps_to_show.is_empty() && app.selected_app >= apps_to_show.len() {
        app.selected_app = apps_to_show.len() - 1;
    }
    
    // --- Categories list ---
    let category_names: Vec<String> = app.categories
        .iter()
        .map(|c| format!("{}  {}", crate::icons::category_icon(c), c))
        .collect();
    
    layout::render_list(
        f,
        categories_area,
        "Categories",
        &category_names,
        app.selected_category,
        app.focus == Focus::Categories,
        config,
    );
    
    // --- Apps list ---
    let selected_index_in_apps = if apps_to_show.is_empty() { 0 } else { app.selected_app };
    
    layout::render_list(
        f,
        apps_area,
        "Apps",
        &apps_to_show,
        selected_index_in_apps,
        app.focus == Focus::Apps,
        config,
    );
}
