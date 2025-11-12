use crate::app::{App, AppEntry, Focus};
use crate::ui::layout;
use crate::config::{LauncherConfig, SearchPosition};
use ratatui::Frame;

/// Draw the dual-pane view with Recent as a category
pub fn draw(f: &mut Frame, app: &mut App, search_position: SearchPosition, config: &LauncherConfig) {
    let (search_area, content_area) = layout::vertical_split(f, 3, search_position);
    layout::render_search_bar(
        f,
        search_area,
        &app.search_query,
        app.focus,
        config,
    );
    let (categories_area, apps_area) = layout::horizontal_split(content_area);
    let query_lower = app.search_query.to_lowercase();
    
    // When searching, filter categories to only show those with matches
    let (categories_to_show, category_indices): (Vec<String>, Vec<usize>) = if !query_lower.is_empty() {
        app.categories
            .iter()
            .enumerate()
            .filter(|(_, cat_name)| {
                if *cat_name == "Recent" {
                    // Check if any recent app matches
                    app.recent_apps.iter().any(|recent_name| {
                        app.apps.iter()
                            .find(|a| &a.name == recent_name)
                            .and_then(|a| app.matches_search(&a.name, &query_lower))
                            .is_some()
                    })
                } else {
                    // Check if any app in this category matches
                    app.apps.iter().any(|a| {
                        &a.category == *cat_name && app.matches_search(&a.name, &query_lower).is_some()
                    })
                }
            })
            .map(|(idx, cat)| (cat.clone(), idx))
            .unzip()
    } else {
        // Not searching: show all categories with their indices
        let cats: Vec<String> = app.categories.clone();
        let indices: Vec<usize> = (0..app.categories.len()).collect();
        (cats, indices)
    };
    
    // Find which position in the filtered list corresponds to app.selected_category
    let display_idx = category_indices.iter()
        .position(|&idx| idx == app.selected_category)
        .unwrap_or(0);
    
    // Clamp to valid range
    let display_idx = display_idx.min(categories_to_show.len().saturating_sub(1));
    
    // Get the actual category name to display apps from
    let selected_category_name = app.categories.get(app.selected_category)
        .cloned()
        .unwrap_or_default();
    
    // Filter apps to display based on the selected category
    let mut apps_to_show: Vec<(AppEntry, i64)> = if selected_category_name == "Recent" {
        // For Recent category, show recent apps that exist and match search
        app.recent_apps
            .iter()
            .filter_map(|recent_name| {
                app.apps.iter()
                    .find(|a| &a.name == recent_name)
                    .cloned()
            })
            .filter_map(|a| app.matches_search(&a.name, &query_lower).map(|score| (a, score)))
            .collect()
    } else {
        // For other categories, filter by category and search
        app.apps
            .iter()
            .filter(|a| a.category == selected_category_name)
            .filter_map(|a| app.matches_search(&a.name, &query_lower).map(|score| (a.clone(), score)))
            .collect()
    };
    
    // Sort by fuzzy score
    apps_to_show.sort_by(|a, b| b.1.cmp(&a.1));
    let apps_to_show: Vec<AppEntry> = apps_to_show.into_iter().map(|(a, _)| a).collect();
    
    // Clamp selected app index
    if !apps_to_show.is_empty() && app.selected_app >= apps_to_show.len() {
        app.selected_app = apps_to_show.len() - 1;
    }
    
    // Render categories (only matching ones when searching)
    let category_names: Vec<String> = categories_to_show
        .iter()
        .map(|c| format!("{}  {}", crate::icons::category_icon(c), c))
        .collect();
    
    let categories_title = " Categories ";
    
    layout::render_list(
        f,
        categories_area,
        categories_title,
        &category_names,
        display_idx,
        app.focus == Focus::Categories,
        config,
    );
    
    // Render apps from the selected category
    let app_names: Vec<String> = apps_to_show.iter().map(|a| a.name.clone()).collect();
    let selected_index_in_apps = if apps_to_show.is_empty() { 0 } else { app.selected_app };
    layout::render_list(
        f,
        apps_area,
        " Apps ",
        &app_names,
        selected_index_in_apps,
        app.focus == Focus::Apps,
        config,
    );
}
