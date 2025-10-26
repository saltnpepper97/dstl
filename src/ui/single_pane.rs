use crate::ui::layout;
use crate::app::{App, Focus};
use crate::config::{LauncherConfig, SearchPosition};
use ratatui::Frame;

pub fn draw(
    f: &mut Frame,
    app: &App,
    _search_query: &str,   // ignore external query; use app.search_query
    _apps: &[String],      // ignore external apps; use app.visible_apps()
    selected: usize,
    focus: Focus,
    search_position: SearchPosition,
    config: &LauncherConfig,
) {
    let chunks = layout::vertical_split(f, 3, search_position);

    // Get apps to display based on recent_first and fuzzy search
    let filtered_apps: Vec<String> = app
        .visible_apps()
        .into_iter()
        .map(|a| a.name.clone())
        .collect();

    // Draw apps list
    layout::render_list(
        f,
        chunks.1,
        " Apps ",
        &filtered_apps,
        selected,
        focus == Focus::Apps,
        config,
    );

    // Draw search bar
    layout::render_search_bar(
        f,
        chunks.0,
        &app.search_query,
        focus,
        config,
    );
}
