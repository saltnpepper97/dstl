use crate::ui::layout;
use crate::app::Focus;
use crate::config::{LauncherConfig, SearchPosition};
use ratatui::Frame;

pub fn draw(
    f: &mut Frame,
    search_query: &str,
    apps: &[String],
    selected: usize,
    focus: Focus,
    search_position: SearchPosition,
    config: &LauncherConfig,
) {
    let chunks = layout::vertical_split(f, 3, search_position);

    // Filter apps according to search query
    let filtered_apps: Vec<String> = if search_query.is_empty() {
        apps.to_vec()
    } else {
        apps.iter()
            .filter(|a| a.to_lowercase().contains(&search_query.to_lowercase()))
            .cloned()
            .collect()
    };

    // Draw apps list — pass config so border color is correct
    layout::render_list(
        f,
        chunks.1,
        "Apps",
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
