use crossterm::event::KeyEvent;
use crate::app::{App, Focus, Mode};
use crate::config::SearchPosition;
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

        // Tab still works for cycling
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

        // Up/Down: Navigate within lists, or move to Search when at top
        Up | Char('k') => {
            match app.mode {
                Mode::SinglePane => {
                    match app.focus {
                        Focus::Search => {
                            // If search bar is at the bottom, moving up should go to list
                            if app.config.search_position == SearchPosition::Bottom {
                                app.focus = Focus::Apps;
                            }
                        }
                        Focus::Apps => {
                            if app.selected_app > 0 {
                                app.selected_app -= 1;
                            } else if app.config.search_position == SearchPosition::Top {
                                // At first item, go back to search only if search is above
                                app.focus = Focus::Search;
                            }
                        }
                        Focus::Categories => {}
                    }
                }

                Mode::DualPane => {
                    match app.focus {
                        Focus::Search => {
                            if app.config.search_position == SearchPosition::Bottom {
                                app.focus = Focus::Categories;
                            }
                        }
                        Focus::Apps => {
                            if app.selected_app > 0 {
                                app.selected_app -= 1;
                            } else if app.config.search_position == SearchPosition::Top {
                                app.focus = Focus::Search;
                            }
                        }
                        Focus::Categories => {
                            // Get previous matching category
                            let matching_categories = get_matching_category_indices(app);
                            if let Some(current_pos) = matching_categories.iter().position(|&idx| idx == app.selected_category) {
                                if current_pos > 0 {
                                    // Move to previous matching category
                                    app.selected_category = matching_categories[current_pos - 1];
                                    app.selected_app = 0;
                                } else if app.config.search_position == SearchPosition::Top {
                                    // At first matching category, go to search
                                    app.focus = Focus::Search;
                                }
                            } else if app.config.search_position == SearchPosition::Top {
                                app.focus = Focus::Search;
                            }
                        }
                    }
                }
            }
        }

        Down | Char('j') => {
            match app.mode {
                Mode::SinglePane => {
                    match app.focus {
                        Focus::Search => {
                            // If search is at top, move down into list
                            if app.config.search_position == SearchPosition::Top {
                                app.focus = Focus::Apps;
                            }
                        }
                        Focus::Apps => {
                            let count = count_filtered_apps_in_current_category(app);
                            if count > 0 && app.selected_app + 1 < count {
                                // Move down within list
                                app.selected_app += 1;
                            } else if app.config.search_position == SearchPosition::Bottom {
                                // At end of list, move to search bar
                                app.focus = Focus::Search;
                            }
                        }
                        _ => {}
                    }
                }

                Mode::DualPane => {
                    match app.focus {
                        Focus::Search => {
                            // If search is at top, go into categories
                            if app.config.search_position == SearchPosition::Top {
                                app.focus = Focus::Categories;
                            }
                        }
                        Focus::Categories => {
                            // Get next matching category
                            let matching_categories = get_matching_category_indices(app);
                            if let Some(current_pos) = matching_categories.iter().position(|&idx| idx == app.selected_category) {
                                if current_pos + 1 < matching_categories.len() {
                                    // Move to next matching category
                                    app.selected_category = matching_categories[current_pos + 1];
                                    app.selected_app = 0;
                                } else if app.config.search_position == SearchPosition::Bottom {
                                    // Last matching category → go to search bar
                                    app.focus = Focus::Search;
                                }
                            } else if app.config.search_position == SearchPosition::Bottom {
                                app.focus = Focus::Search;
                            }
                        }
                        Focus::Apps => {
                            let count = count_filtered_apps_in_current_category(app);
                            if count > 0 && app.selected_app + 1 < count {
                                // Move down within apps
                                app.selected_app += 1;
                            } else if app.config.search_position == SearchPosition::Bottom {
                                // At end of list, move to search bar
                                app.focus = Focus::Search;
                            }
                        }
                    }
                }
            }
        }

        // Left/Right: Navigate within lists when focused on them
        Left | Char('h') => {
            match app.focus {
                Focus::Apps => {
                    if app.mode == Mode::DualPane {
                        app.focus = Focus::Categories;
                    } else {
                        // In apps list, move selection up
                        if app.selected_app > 0 {
                            app.selected_app -= 1;
                        }
                    }
                }
                Focus::Categories => {
                    // Get previous matching category
                    let matching_categories = get_matching_category_indices(app);
                    if let Some(current_pos) = matching_categories.iter().position(|&idx| idx == app.selected_category) {
                        if current_pos > 0 {
                            app.selected_category = matching_categories[current_pos - 1];
                            app.selected_app = 0;
                        }
                    }
                }
                _ => {}
            }
        }

        Right | Char('l') => {
            match app.focus {
                Focus::Categories => {
                    if app.mode == Mode::DualPane {
                        // Move between categories and apps
                        app.focus = Focus::Apps;
                    } else {
                        // Get next matching category
                        let matching_categories = get_matching_category_indices(app);
                        if let Some(current_pos) = matching_categories.iter().position(|&idx| idx == app.selected_category) {
                            if current_pos + 1 < matching_categories.len() {
                                app.selected_category = matching_categories[current_pos + 1];
                                app.selected_app = 0;
                            }
                        }
                    }
                }
                Focus::Apps => {
                    // Navigate down in apps list
                    let count = count_filtered_apps_in_current_category(app);
                    if count > 0 && app.selected_app + 1 < count {
                        app.selected_app += 1;
                    }
                }
                _ => {}
            }
        }

        _ => {}
    }

    Ok(false)
}

/// Get indices of categories that have matching apps (or all if no search)
fn get_matching_category_indices(app: &App) -> Vec<usize> {
    if app.search_query.is_empty() {
        // No search: return all category indices
        (0..app.categories.len()).collect()
    } else {
        // Search active: return only categories with matches
        let query_lower = app.search_query.to_lowercase();
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
            .map(|(idx, _)| idx)
            .collect()
    }
}

/// Updates selected app and category when search query changes
fn update_selection_after_search(app: &mut App) {
    if app.search_query.is_empty() {
        app.selected_category = 0;
        app.selected_app = 0;
        return;
    }

    match app.mode {
        Mode::DualPane => {
            // Find first matching category
            let matching_indices = get_matching_category_indices(app);
            if let Some(&first_match) = matching_indices.first() {
                app.selected_category = first_match;
                app.selected_app = 0;
            }
        }
        Mode::SinglePane => { app.selected_app = 0; }
    }
}

/// Return currently selected app - MUST match UI sorting logic
fn get_selected_app(app: &App) -> Option<&crate::app::AppEntry> {
    match app.mode {
        Mode::SinglePane => {
            // Use visible_apps() so recent_first ordering is respected
            app.visible_apps().get(app.selected_app).map(|v| &**v)
        }
        Mode::DualPane => {
            let cat_name = app.categories.get(app.selected_category)?;
            
            if cat_name == "Recent" {
                // For Recent: maintain order from recent_apps list
                let apps_in_order: Vec<&crate::app::AppEntry> = app.recent_apps.iter()
                    .filter_map(|recent_name| {
                        app.apps.iter().find(|a| &a.name == recent_name)
                    })
                    .collect();
                
                // If searching, filter and sort by score
                if !app.search_query.is_empty() {
                    let mut apps_with_scores: Vec<(&crate::app::AppEntry, i64)> = apps_in_order
                        .into_iter()
                        .filter_map(|a| app.matches_search(&a.name, &app.search_query).map(|score| (a, score)))
                        .collect();
                    apps_with_scores.sort_by(|a, b| b.1.cmp(&a.1));
                    return apps_with_scores.get(app.selected_app).map(|(entry, _)| *entry);
                }
                
                // No search: return in recent order
                apps_in_order.get(app.selected_app).copied()
            } else {
                // For other categories: filter by category
                let mut apps_with_scores: Vec<(&crate::app::AppEntry, i64)> = app.apps.iter()
                    .filter(|a| &a.category == cat_name)
                    .filter_map(|a| app.matches_search(&a.name, &app.search_query).map(|score| (a, score)))
                    .collect();

                if !app.search_query.is_empty() {
                    apps_with_scores.sort_by(|a, b| b.1.cmp(&a.1));
                }

                apps_with_scores.get(app.selected_app).map(|(entry, _)| *entry)
            }
        }
    }
}

/// Count filtered apps for navigation in the currently selected category - MUST match UI
fn count_filtered_apps_in_current_category(app: &App) -> usize {
    match app.mode {
        Mode::SinglePane => {
            app.visible_apps().len()
        }
        Mode::DualPane => {
            let cat_name = match app.categories.get(app.selected_category) {
                Some(c) => c,
                None => return 0,
            };
            
            if cat_name == "Recent" {
                // Count recent apps that exist and match search
                app.recent_apps.iter()
                    .filter_map(|recent_name| {
                        app.apps.iter().find(|a| &a.name == recent_name)
                    })
                    .filter(|a| app.matches_search(&a.name, &app.search_query).is_some())
                    .count()
            } else {
                app.apps.iter()
                    .filter(|a| &a.category == cat_name)
                    .filter(|a| app.matches_search(&a.name, &app.search_query).is_some())
                    .count()
            }
        }
    }
}
