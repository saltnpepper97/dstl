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
            (chunks[0], chunks[1])
        }
        SearchPosition::Bottom => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(search_height)])
                .split(full_area);
            (chunks[1], chunks[0])
        }
    }
}

pub fn horizontal_split(area: Rect) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Percentage(70)])
        .split(area);
    (chunks[0], chunks[1])
}


pub fn render_search_bar(
    f: &mut Frame,
    area: Rect,
    query: &str,
    cursor_position: usize,
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
        .title(" Search ")
        .borders(Borders::ALL)
        .border_type(LauncherTheme::parse_border_type(&config.colors.border_style))
        .border_style(Style::default().fg(border_color));
    
    // Calculate horizontal scroll offset based on cursor position
    let available_width = area.width.saturating_sub(4) as usize; // 2 for borders, 2 for padding
    let query_chars: Vec<char> = query.chars().collect();
    let query_len = query_chars.len();
    
    let horizontal_offset = if cursor_position > available_width {
        cursor_position - available_width
    } else {
        0
    };
    
    // Determine if we need ellipsis
    let show_left_ellipsis = horizontal_offset > 0;
    let total_remaining = query_len - horizontal_offset;
    let show_right_ellipsis = total_remaining > available_width;
    
    // Calculate actual text window (accounting for ellipsis)
    let text_width = if show_left_ellipsis && show_right_ellipsis {
        available_width.saturating_sub(2) // Both ellipsis
    } else if show_left_ellipsis || show_right_ellipsis {
        available_width.saturating_sub(1) // One ellipsis
    } else {
        available_width
    };
    
    // Extract visible portion
    let visible_start = horizontal_offset;
    let visible_end = (visible_start + text_width).min(query_len);
    let visible_text: String = query_chars[visible_start..visible_end].iter().collect();
    
    // Build display text with ellipsis
    let mut display_text = String::from(" ");
    if show_left_ellipsis {
        display_text.push('…');
    }
    display_text.push_str(&visible_text);
    if show_right_ellipsis {
        display_text.push('…');
    }
    display_text.push(' ');
    
    let paragraph = Paragraph::new(display_text)
        .block(block.clone())
        .style(Style::default().fg(text_color));
    
    f.render_widget(paragraph, area);
    // Note: cursor position is set in main.rs run_app() function
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
        .map(|a| ListItem::new(format!(" {} ", a)))
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
