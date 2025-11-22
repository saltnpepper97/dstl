use crate::app::{App, AppEntry, Focus};
use crate::ui::layout;
use crate::config::{LauncherConfig, SearchPosition};
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &mut App, search_position: SearchPosition, config: &LauncherConfig) {
    let (search_area, content_area) = layout::vertical_split(f, 3, search_position);
    
    // Pass cursor_position to render_search_bar
    layout::render_search_bar(
        f,
        search_area,
        &app.search_query,
        app.cursor_position,
        app.focus,
        config,
    );
    
    let (categories_area, apps_area) = layout::horizontal_split(content_area);
    let query_lower = app.search_query.to_lowercase();
    
    let (categories_to_show, category_indices): (Vec<String>, Vec<usize>) = if !query_lower.is_empty() {
        app.categories
            .iter()
            .enumerate()
            .filter(|(_, cat_name)| {
                if *cat_name == "Recent" {
                    app.recent_apps.iter().any(|recent_name| {
                        app.apps.iter()
                            .find(|a| &a.name == recent_name)
                            .and_then(|a| app.matches_search(&a.name, &query_lower))
                            .is_some()
                    })
                } else {
                    app.apps.iter().any(|a| {
                        &a.category == *cat_name && app.matches_search(&a.name, &query_lower).is_some()
                    })
                }
            })
            .map(|(idx, cat)| (cat.clone(), idx))
            .unzip()
    } else {
        let cats: Vec<String> = app.categories.clone();
        let indices: Vec<usize> = (0..app.categories.len()).collect();
        (cats, indices)
    };
    
    let display_idx = category_indices.iter()
        .position(|&idx| idx == app.selected_category)
        .unwrap_or(0);
    
    let display_idx = display_idx.min(categories_to_show.len().saturating_sub(1));
    
    let selected_category_name = app.categories.get(app.selected_category)
        .cloned()
        .unwrap_or_default();
    
    let mut apps_to_show: Vec<(AppEntry, i64)> = if selected_category_name == "Recent" {
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
        app.apps
            .iter()
            .filter(|a| a.category == selected_category_name)
            .filter_map(|a| app.matches_search(&a.name, &query_lower).map(|score| (a.clone(), score)))
            .collect()
    };
    
    apps_to_show.sort_by(|a, b| b.1.cmp(&a.1));
    let apps_to_show: Vec<AppEntry> = apps_to_show.into_iter().map(|(a, _)| a).collect();
    
    if !apps_to_show.is_empty() && app.selected_app >= apps_to_show.len() {
        app.selected_app = apps_to_show.len() - 1;
    }
    
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
