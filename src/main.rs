mod app;
mod config;
mod events;
mod icons;
mod launch;
mod ui;

use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::{
    io::{self, Write},
    time::{Duration, Instant},
};
use crossterm::{
    cursor::{MoveTo, SetCursorStyle},
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use eyre::Result;

use app::{App, Focus, Mode, SinglePaneMode};
use config::{load_launcher_config, CursorShape, SearchPosition};

fn main() -> Result<()> {
    color_eyre::install()?;

    let cfg = load_launcher_config();

    let single_pane_mode = if cfg.dmenu {
        SinglePaneMode::Dmenu
    } else {
        SinglePaneMode::DesktopApps
    };

    let start_mode = match cfg.start_mode {
        config::StartMode::Dual => Mode::DualPane,
        config::StartMode::Single => Mode::SinglePane,
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    
    // Set cursor color using ANSI escape codes
    set_cursor_color(&mut stdout, &cfg.colors.cursor_color)?;
    
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(single_pane_mode, start_mode, &cfg);

    warmup_icons(&mut terminal, &app, &cfg)?;

    if start_mode == Mode::DualPane && !app.categories.is_empty() {
        let old_focus = app.focus;
        app.focus = Focus::Categories;
        terminal.draw(|f| ui::draw(f, &mut app, cfg.search_position.clone(), &cfg))?;
        app.focus = old_focus;
    }

    let res = run_app(&mut terminal, &mut app, &cfg);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    // Reset cursor color to default
    reset_cursor_color(terminal.backend_mut())?;

    if let Err(err) = res {
        eprintln!("Error: {err:?}");
    }

    if let Some(ref cmd) = app.app_to_launch {
        if let Some(entry) = app.apps.iter().find(|a| &a.exec == cmd).cloned() {
            app.add_to_recent(entry.name.clone());
            crate::launch::launch_app(&entry, &app.config);
        } else {
            let _ = std::process::Command::new("sh").arg("-c").arg(cmd).spawn();
        }
    }

    Ok(())
}

/// Set the cursor color using ANSI escape codes
fn set_cursor_color<W: Write>(writer: &mut W, color_hex: &str) -> Result<()> {
    if let Some((r, g, b)) = parse_hex_color(color_hex) {
        // OSC 12 ; color ST - Set cursor color
        write!(writer, "\x1b]12;rgb:{:02x}/{:02x}/{:02x}\x07", r, g, b)?;
        writer.flush()?;
    }
    Ok(())
}

/// Reset cursor color to terminal default
fn reset_cursor_color<W: Write>(writer: &mut W) -> Result<()> {
    // OSC 112 ST - Reset cursor color
    write!(writer, "\x1b]112\x07")?;
    writer.flush()?;
    Ok(())
}

/// Parse hex color string to RGB values
fn parse_hex_color(color: &str) -> Option<(u8, u8, u8)> {
    let color = color.trim();
    
    if !color.starts_with('#') {
        return None;
    }
    
    let hex = &color[1..];
    
    match hex.len() {
        // #RGB format
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
            Some((r * 17, g * 17, b * 17))
        }
        // #RRGGBB format
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some((r, g, b))
        }
        // #RRGGBBAA format (ignore alpha)
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some((r, g, b))
        }
        _ => None,
    }
}

fn run_app<B: Backend + ExecutableCommand>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    cfg: &config::LauncherConfig,
) -> Result<()> {
    let mut last_input = Instant::now();

    loop {
        app.update_cursor_blink();

        terminal.draw(|f| ui::draw(f, app, cfg.search_position.clone(), cfg))?;

        if app.focus == Focus::Search {
            let frame = terminal.get_frame();
            let full_area = frame.area();

            let search_area = match cfg.search_position {
                SearchPosition::Top => ratatui::layout::Rect::new(full_area.x, full_area.y, full_area.width, 3),
                SearchPosition::Bottom => ratatui::layout::Rect::new(
                    full_area.x,
                    full_area.height.saturating_sub(3),
                    full_area.width,
                    3,
                ),
            };

            // Calculate scroll offset to match the Paragraph widget
            let available_width = search_area.width.saturating_sub(4) as usize;
            let horizontal_offset = if app.cursor_position > available_width {
                app.cursor_position - available_width
            } else {
                0
            };
            let visible_cursor_pos = app.cursor_position - horizontal_offset;
            let cursor_x = search_area.x + 2 + visible_cursor_pos as u16;
            let cursor_y = search_area.y + 1;

            let backend = terminal.backend_mut();

            
            // Move cursor
            backend.execute(MoveTo(cursor_x, cursor_y))?;

            // Set shape based on blink interval
            let style = if cfg.colors.cursor_blink_interval > 0 {
                // Use steady cursor - we'll handle blinking manually
                match cfg.colors.cursor_shape {
                    CursorShape::Block => SetCursorStyle::SteadyBlock,
                    CursorShape::Underline => SetCursorStyle::SteadyUnderScore,
                    CursorShape::Pipe => SetCursorStyle::SteadyBar,
                }
            } else {
                // Use terminal's built-in blinking
                match cfg.colors.cursor_shape {
                    CursorShape::Block => SetCursorStyle::BlinkingBlock,
                    CursorShape::Underline => SetCursorStyle::BlinkingUnderScore,
                    CursorShape::Pipe => SetCursorStyle::BlinkingBar,
                }
            };
            backend.execute(style)?;

            // Handle manual cursor blinking if interval is set
            if cfg.colors.cursor_blink_interval > 0 {
                if app.cursor_visible {
                    terminal.show_cursor()?;
                } else {
                    terminal.hide_cursor()?;
                }
            } else {
                terminal.show_cursor()?;
            }

        } else {
            terminal.hide_cursor()?;
        }

        let tick = Duration::from_millis(50);

        if cfg.timeout > 0 && last_input.elapsed().as_secs() >= cfg.timeout {
            break;
        }

        if event::poll(tick)? {
            if let Event::Key(key) = event::read()? {
                last_input = Instant::now();
                if events::handle_key(app, key)? {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn warmup_icons<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &App,
    cfg: &config::LauncherConfig,
) -> Result<()> {
    if app.categories.is_empty() {
        return Ok(());
    }

    let mut tmp = app.clone();
    tmp.focus = Focus::Apps;
    terminal.draw(|f| ui::draw(f, &mut tmp, cfg.search_position.clone(), cfg))?;

    if app.mode == Mode::DualPane {
        tmp.focus = Focus::Categories;
        terminal.draw(|f| ui::draw(f, &mut tmp, cfg.search_position.clone(), cfg))?;
    }

    Ok(())
}
