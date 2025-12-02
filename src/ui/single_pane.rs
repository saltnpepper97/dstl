use crate::ui::layout;
use crate::app::{App, Focus};
use crate::config::{DstlConfig, SearchPosition};
use ratatui::Frame;

pub fn draw(
    f: &mut Frame,
    app: &App,
    _search_query: &str,
    _apps: &[String],
    selected: usize,
    focus: Focus,
    search_position: SearchPosition,
    config: &DstlConfig,
) {
    let chunks = layout::vertical_split(f, 3, search_position);
    
    let filtered_apps: Vec<String> = app
        .visible_apps()
        .into_iter()
        .map(|a| a.name.clone())
        .collect();
    
    layout::render_list(
        f,
        chunks.1,
        " Apps ",
        &filtered_apps,
        selected,
        focus == Focus::Apps,
        config,
    );
    
    // Pass cursor_position to render_search_bar
    layout::render_search_bar(
        f,
        chunks.0,
        &app.search_query,
        app.cursor_position,
        focus,
        config,
    );
}
