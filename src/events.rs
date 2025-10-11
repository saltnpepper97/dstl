use crossterm::event::KeyEvent;
use crate::app::{App, Focus, Mode};
use eyre::Result;

pub fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    use crossterm::event::KeyCode::*;

    match key.code {
        Esc => return Ok(true), // Esc always quits
        Char('q') if app.focus != Focus::Search => return Ok(true), // Quit only if not in search
        Char('q') if app.focus == Focus::Search => {
            app.search_query.push('q'); // Treat 'q' as input in search
            update_selection_after_search(app);
        }

        // Enter launches the selected app
        Enter => {
            if let Some(app_entry) = get_selected_app(app) {
                app.app_to_launch = Some(app_entry.exec.clone());
                app.should_quit = true;
                return Ok(true);
            }
        }

        Char('m') if app.focus != Focus::Search => {
            app.toggle_mode();

            // optionally focus search if enabled
            if app.config.focus_search_on_switch {
                app.focus = Focus::Search;
            }
        }

        // Search input
        Char(c) if app.focus == Focus::Search => {
            app.search_query.push(c);
            update_selection_after_search(app);
        }

        Backspace if app.focus == Focus::Search => {
            app.search_query.pop();
            update_selection_after_search(app);
        }

        // Cycle focus
        Tab => {
            app.focus = match app.mode {
                Mode::SinglePane => match app.focus {
                    Focus::Search => Focus::Apps,
                    Focus::Apps | Focus::Categories => Focus::Search,
                },
                Mode::DualPane => match app.focus {
                    Focus::Search => Focus::Categories,
                    Focus::Categories => Focus::Apps,
                    Focus::Apps => Focus::Search,
                },
            };
        }

        // Up/Down navigation
        Up | Char('k') if app.focus != Focus::Search => {
            match app.focus {
                Focus::Apps => {
                    if app.selected_app > 0 {
                        app.selected_app -= 1;
                    }
                }
                Focus::Categories => {
                    if app.selected_category > 0 {
                        app.selected_category -= 1;
                        app.selected_app = 0;
                    }
                }
                _ => {}
            }
        }

        Down | Char('j') if app.focus != Focus::Search => {
            match app.focus {
                Focus::Apps => {
                    let count = count_filtered_apps_in_current_category(app);
                    if count > 0 && app.selected_app + 1 < count {
                        app.selected_app += 1;
                    }
                }
                Focus::Categories => {
                    if app.selected_category + 1 < app.categories.len() {
                        app.selected_category += 1;
                        app.selected_app = 0;
                    }
                }
                _ => {}
            }
        }

        // Left/Right only in dual-pane
        Left | Char('h') if app.mode == Mode::DualPane && app.focus == Focus::Apps => {
            app.focus = Focus::Categories;
        }

        Right | Char('l') if app.mode == Mode::DualPane && app.focus == Focus::Categories => {
            app.focus = Focus::Apps;
        }

        _ => {}
    }

    Ok(false)
}

/// Updates selected app and category when search query changes
fn update_selection_after_search(app: &mut App) {
    if app.search_query.is_empty() {
        app.selected_category = 0;
        app.selected_app = 0;
        return;
    }

    let query = app.search_query.to_lowercase();
    
    match app.mode {
        Mode::DualPane => {
            // In dual-pane, find first matching app across all categories
            for (cat_idx, cat_name) in app.categories.iter().enumerate() {
                let apps_in_cat: Vec<_> = app.apps.iter()
                    .filter(|a| &a.category == cat_name && a.name.to_lowercase().contains(&query))
                    .collect();

                if !apps_in_cat.is_empty() {
                    app.selected_category = cat_idx;
                    app.selected_app = 0;
                    return;
                }
            }
            // No match found - leave selection as is
        }
        Mode::SinglePane => {
            // In single-pane, reset to first match
            app.selected_app = 0;
        }
    }
}

/// Return currently selected app
fn get_selected_app(app: &App) -> Option<&crate::app::AppEntry> {
    match app.mode {
        Mode::DualPane => {
            let cat_name = app.categories.get(app.selected_category)?;
            let query = app.search_query.to_lowercase();
            let filtered: Vec<_> = app.apps.iter()
                .filter(|a| &a.category == cat_name && 
                    (app.search_query.is_empty() || a.name.to_lowercase().contains(&query)))
                .collect();
            filtered.get(app.selected_app).copied()
        }
        Mode::SinglePane => {
            let query = app.search_query.to_lowercase();
            let filtered: Vec<_> = app.apps.iter()
                .filter(|a| app.search_query.is_empty() || a.name.to_lowercase().contains(&query))
                .collect();
            filtered.get(app.selected_app).copied()
        }
    }
}

/// Count filtered apps for navigation in the currently selected category
fn count_filtered_apps_in_current_category(app: &App) -> usize {
    match app.mode {
        Mode::DualPane => {
            let cat_name = match app.categories.get(app.selected_category) {
                Some(c) => c,
                None => return 0,
            };

            let query = app.search_query.to_lowercase();
            app.apps.iter()
                .filter(|a| &a.category == cat_name && 
                    (app.search_query.is_empty() || a.name.to_lowercase().contains(&query)))
                .count()
        }
        Mode::SinglePane => {
            if app.search_query.is_empty() {
                app.apps.len()
            } else {
                let query = app.search_query.to_lowercase();
                app.apps.iter()
                    .filter(|a| a.name.to_lowercase().contains(&query))
                    .count()
            }
        }
    }
}
