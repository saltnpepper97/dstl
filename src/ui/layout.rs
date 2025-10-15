use ratatui::{
    Frame,
    layout::{Layout, Constraint, Direction, Rect},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    style::{Style, Color},
};
use crate::app::Focus;
use crate::config::{LauncherConfig, LauncherTheme, SearchPosition};

pub fn vertical_split(f: &Frame, search_height: u16, search_position: SearchPosition) -> (Rect, Rect) {
    let full_area = f.area();

    match search_position {
        SearchPosition::Top => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(search_height), Constraint::Min(0)])
                .split(full_area);
            (chunks[0], chunks[1]) // search on top, content below
        }
        SearchPosition::Bottom => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(search_height)])
                .split(full_area);
            (chunks[1], chunks[0]) // search at bottom, content above
        }
    }
}

pub fn horizontal_split(area: Rect) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(20),      // left pane: categories/icons
            Constraint::Percentage(70), // right pane: apps/content
        ])
        .split(area);

    (chunks[0], chunks[1])
}

pub fn render_search_bar(
    f: &mut Frame,
    area: Rect,
    query: &str,
    focus: Focus,
    config: &LauncherConfig,
) {
    let border_color = if focus == Focus::Search {
        LauncherTheme::parse_color(&config.colors.focus)
    } else {
        LauncherTheme::parse_color(&config.colors.border)
    };

    let text_color = border_color;

    let block = Block::default()
        .title("Search")
        .borders(Borders::ALL)
        .border_type(LauncherTheme::parse_border_type(&config.colors.border_style))
        .border_style(Style::default().fg(border_color));

    // Add padding spaces around text
    let padded_query = format!(" {} ", query);

    // Compute the visible width inside the box (minus borders)
    let inner_width = area.width.saturating_sub(2); // minus left/right borders

    // Determine horizontal scroll offset
    // If query is longer than visible width, scroll so the end of the text is visible
    let scroll_offset = if padded_query.len() as u16 > inner_width {
        padded_query.len() as u16 - inner_width
    } else {
        0
    };

    let paragraph = Paragraph::new(padded_query)
        .block(block)
        .style(Style::default().fg(text_color))
        .scroll((0, scroll_offset));

    f.render_widget(paragraph, area);
}


pub fn render_list(
    f: &mut Frame,
    area: Rect,
    title: &str,
    items: &[String],
    selected: usize,
    focus_on_title: bool,
    config: &LauncherConfig,
) {
    let mut state = ListState::default();
    let sel = if selected >= items.len() { 0 } else { selected };
    state.select(Some(sel));

    let border_color = if focus_on_title {
        LauncherTheme::parse_color(&config.colors.focus)
    } else {
        LauncherTheme::parse_color(&config.colors.border)
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(LauncherTheme::parse_border_type(&config.colors.border_style))
        .border_style(Style::default().fg(border_color));
   
    let list_items: Vec<ListItem> = items.iter()
        .map(|a| {
            // Add 1 space on each side
            let padded = format!(" {} ", a);
            ListItem::new(padded)
        })
        .collect();
 
    let highlight_color = LauncherTheme::parse_color(&config.colors.highlight);
    let highlight_style = match config.colors.highlight_type.to_lowercase().as_str() {
        "foreground" => Style::default().fg(highlight_color),
        "background" | _ => Style::default().bg(highlight_color).fg(Color::Black),
    };

    let list = List::new(list_items)
        .block(block)
        .highlight_style(highlight_style)
        .highlight_symbol("");

    f.render_stateful_widget(list, area, &mut state);
}

