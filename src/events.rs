use crossterm::event::KeyEvent;
use crate::app::{App, Focus, Mode};
use crate::config::SearchPosition;
use eyre::Result;

pub fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    use crossterm::event::KeyCode::*;

    match key.code {
        Esc => return Ok(true),
        Char('q') if app.focus != Focus::Search => return Ok(true),
        Char('q') if app.focus == Focus::Search => {
            // Insert 'q' at cursor position
            let pos = app.cursor_position.min(app.search_query.len());
            app.search_query.insert(pos, 'q');
            app.cursor_position += 1;
            update_selection_after_search(app);
        }

        Enter => {
            if let Some(app_entry) = get_selected_app(app) {
                app.app_to_launch = Some(app_entry.exec.clone());
                app.should_quit = true;
                return Ok(true);
            }
        }

        Char('m') if app.focus != Focus::Search => {
            app.toggle_mode();
            if app.config.focus_search_on_switch {
                app.focus = Focus::Search;
            }
        }

        // Left/Right arrow keys for cursor movement in search
        Left if app.focus == Focus::Search => {
            if app.cursor_position > 0 {
                app.cursor_position -= 1;
                app.reset_cursor_blink(); // Keep cursor solid while moving
            }
        }

        Left if app.focus != Focus::Search => {
            if app.mode == Mode::DualPane {
                if app.focus == Focus::Apps {
                    app.focus = Focus::Categories;
                }
            }
        }

        Right if app.focus == Focus::Search => {
            let query_len = app.search_query.chars().count();
            if app.cursor_position < query_len {
                app.cursor_position += 1;
                app.reset_cursor_blink(); // Keep cursor solid while moving
            }
        }

        Right if app.focus != Focus::Search => {
            if app.mode == Mode::DualPane {
                if app.focus == Focus::Categories {
                    app.focus = Focus::Apps;
                }
            }
        }

        // Home/End keys for jumping to start/end
        Home if app.focus == Focus::Search => {
            app.cursor_position = 0;
            app.reset_cursor_blink(); // Keep cursor solid while moving
        }

        End if app.focus == Focus::Search => {
            app.cursor_position = app.search_query.chars().count();
            app.reset_cursor_blink(); // Keep cursor solid while moving
        }

        // Search input - insert at cursor position
        Char(c) if app.focus == Focus::Search => {
            let query_chars: Vec<char> = app.search_query.chars().collect();
            let pos = app.cursor_position.min(query_chars.len());
            
            // Reconstruct string with new character inserted
            let before: String = query_chars.iter().take(pos).collect();
            let after: String = query_chars.iter().skip(pos).collect();
            app.search_query = format!("{}{}{}", before, c, after);
            
            app.cursor_position += 1;
            app.reset_cursor_blink(); // Keep cursor solid while typing
            update_selection_after_search(app);
        }

        Backspace if app.focus == Focus::Search => {
            if app.cursor_position > 0 {
                let query_chars: Vec<char> = app.search_query.chars().collect();
                let pos = app.cursor_position - 1;
                
                if pos < query_chars.len() {
                    // Reconstruct string without character at pos
                    let before: String = query_chars.iter().take(pos).collect();
                    let after: String = query_chars.iter().skip(pos + 1).collect();
                    app.search_query = format!("{}{}", before, after);
                    
                    app.cursor_position -= 1;
                    app.reset_cursor_blink(); // Keep cursor solid while deleting
                    update_selection_after_search(app);
                }
            }
        }

        // Delete key to remove character at cursor
        Delete if app.focus == Focus::Search => {
            let query_chars: Vec<char> = app.search_query.chars().collect();
            if app.cursor_position < query_chars.len() {
                // Reconstruct string without character at cursor_position
                let before: String = query_chars.iter().take(app.cursor_position).collect();
                let after: String = query_chars.iter().skip(app.cursor_position + 1).collect();
                app.search_query = format!("{}{}", before, after);
                
                app.reset_cursor_blink(); // Keep cursor solid while deleting
                update_selection_after_search(app);
            }
        }

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
        Up | Char('k') => {
            if app.focus == Focus::Search {
                // Allow Up from search to go to list only if search is at bottom
                if app.config.search_position == SearchPosition::Bottom {
                    app.focus = match app.mode {
                        Mode::SinglePane => Focus::Apps,
                        Mode::DualPane => Focus::Apps,
                    };
                }
            } else {
                // In list navigation
                match app.mode {
                    Mode::SinglePane => {
                        match app.focus {
                            Focus::Apps => {
                                if app.selected_app > 0 {
                                    app.selected_app -= 1;
                                } else if app.config.search_position == SearchPosition::Top {
                                    app.focus = Focus::Search;
                                }
                            }
                            _ => {}
                        }
                    }

                    Mode::DualPane => {
                        match app.focus {
                            Focus::Apps => {
                                if app.selected_app > 0 {
                                    app.selected_app -= 1;
                                } else if app.config.search_position == SearchPosition::Top {
                                    app.focus = Focus::Search;
                                }
                            }
                            Focus::Categories => {
                                let matching_categories = get_matching_category_indices(app);
                                if let Some(current_pos) = matching_categories.iter().position(|&idx| idx == app.selected_category) {
                                    if current_pos > 0 {
                                        app.selected_category = matching_categories[current_pos - 1];
                                        app.selected_app = 0;
                                    } else if app.config.search_position == SearchPosition::Top {
                                        app.focus = Focus::Search;
                                    }
                                } else if app.config.search_position == SearchPosition::Top {
                                    app.focus = Focus::Search;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Down | Char('j') => {
            if app.focus == Focus::Search {
                // Allow Down from search to go to list only if search is at top
                if app.config.search_position == SearchPosition::Top {
                    app.focus = match app.mode {
                        Mode::SinglePane => Focus::Apps,
                        Mode::DualPane => Focus::Categories,
                    };
                }
            } else {
                // In list navigation
                match app.mode {
                    Mode::SinglePane => {
                        match app.focus {
                            Focus::Apps => {
                                let count = count_filtered_apps_in_current_category(app);
                                if count > 0 && app.selected_app + 1 < count {
                                    app.selected_app += 1;
                                } else if app.config.search_position == SearchPosition::Bottom {
                                    app.focus = Focus::Search;
                                }
                            }
                            _ => {}
                        }
                    }

                    Mode::DualPane => {
                        match app.focus {
                            Focus::Categories => {
                                let matching_categories = get_matching_category_indices(app);
                                if let Some(current_pos) = matching_categories.iter().position(|&idx| idx == app.selected_category) {
                                    if current_pos + 1 < matching_categories.len() {
                                        app.selected_category = matching_categories[current_pos + 1];
                                        app.selected_app = 0;
                                    } else if app.config.search_position == SearchPosition::Bottom {
                                        app.focus = Focus::Search;
                                    }
                                } else if app.config.search_position == SearchPosition::Bottom {
                                    app.focus = Focus::Search;
                                }
                            }
                            Focus::Apps => {
                                let count = count_filtered_apps_in_current_category(app);
                                if count > 0 && app.selected_app + 1 < count {
                                    app.selected_app += 1;
                                } else if app.config.search_position == SearchPosition::Bottom {
                                    app.focus = Focus::Search;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // h/l keys only work for list navigation when NOT in search
        Char('h') if app.focus != Focus::Search => {
            match app.focus {
                Focus::Apps => {
                    if app.mode == Mode::DualPane {
                        app.focus = Focus::Categories;
                    } else {
                        if app.selected_app > 0 {
                            app.selected_app -= 1;
                        }
                    }
                }
                Focus::Categories => {
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

        Char('l') if app.focus != Focus::Search => {
            match app.focus {
                Focus::Categories => {
                    if app.mode == Mode::DualPane {
                        app.focus = Focus::Apps;
                    } else {
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

fn get_matching_category_indices(app: &App) -> Vec<usize> {
    if app.search_query.is_empty() {
        (0..app.categories.len()).collect()
    } else {
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

fn update_selection_after_search(app: &mut App) {
    if app.search_query.is_empty() {
        app.selected_category = 0;
        app.selected_app = 0;
        return;
    }

    match app.mode {
        Mode::DualPane => {
            let matching_indices = get_matching_category_indices(app);
            if let Some(&first_match) = matching_indices.first() {
                app.selected_category = first_match;
                app.selected_app = 0;
            }
        }
        Mode::SinglePane => { app.selected_app = 0; }
    }
}

fn get_selected_app(app: &App) -> Option<&crate::app::AppEntry> {
    match app.mode {
        Mode::SinglePane => {
            app.visible_apps().get(app.selected_app).map(|v| &**v)
        }
        Mode::DualPane => {
            let cat_name = app.categories.get(app.selected_category)?;
            
            if cat_name == "Recent" {
                let apps_in_order: Vec<&crate::app::AppEntry> = app.recent_apps.iter()
                    .filter_map(|recent_name| {
                        app.apps.iter().find(|a| &a.name == recent_name)
                    })
                    .collect();
                
                if !app.search_query.is_empty() {
                    let mut apps_with_scores: Vec<(&crate::app::AppEntry, i64)> = apps_in_order
                        .into_iter()
                        .filter_map(|a| app.matches_search(&a.name, &app.search_query).map(|score| (a, score)))
                        .collect();
                    apps_with_scores.sort_by(|a, b| b.1.cmp(&a.1));
                    return apps_with_scores.get(app.selected_app).map(|(entry, _)| *entry);
                }
                
                apps_in_order.get(app.selected_app).copied()
            } else {
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
