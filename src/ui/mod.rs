use crate::app::{App, Mode};
use crate::config::{DstlConfig, SearchPosition};
use ratatui::Frame;

mod layout;
mod dual_pane;
mod single_pane;

pub fn draw(f: &mut Frame, app: &mut App, search_position: SearchPosition, config: &DstlConfig) {
    match app.mode {
        Mode::SinglePane => {
            // Collect app names for single pane
            let app_names: Vec<String> = app.apps.iter().map(|entry| entry.name.clone()).collect();
            single_pane::draw(
                f,
                app,  // Pass app reference
                &app.search_query,
                &app_names,
                app.selected_app,
                app.focus,
                search_position,
                config,
            )
        }
        Mode::DualPane => dual_pane::draw(f, app, search_position, config),
    }
}
