mod app;
mod config;
mod events;
mod icons;
mod launch;
mod ui;

use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::{io, time::{Duration, Instant}};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use eyre::Result;
use app::{App, SinglePaneMode, Mode, Focus};
use config::{load_launcher_config, StartMode};

fn main() -> Result<()> {
    // Enable colorized error reporting
    color_eyre::install()?;
    
    let cfg = load_launcher_config();
    
    let single_pane_mode = if cfg.dmenu {
        SinglePaneMode::Dmenu
    } else {
        SinglePaneMode::DesktopApps
    };
    
    let start_mode = match cfg.start_mode {
        StartMode::Dual => Mode::DualPane,
        StartMode::Single => Mode::SinglePane,
    };
    
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    let mut app = App::new(single_pane_mode, start_mode, &cfg);
    warmup_icons(&mut terminal, &app, &cfg)?;

    // --- First-frame warmup for category icons ---
    if !app.categories.is_empty() {
        let old_mode = app.mode;
        let old_focus = app.focus;

        app.mode = Mode::DualPane;
        app.focus = Focus::Categories;

        terminal.draw(|f| ui::draw(f, &mut app, cfg.search_position.clone(), &cfg))?;

        app.mode = old_mode;
        app.focus = old_focus;
    }

    // Main loop
    let res = run_app(&mut terminal, &mut app, &cfg.search_position, &cfg);
    
    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }
    
    // Launch selected app (if any)
    if let Some(ref command) = app.app_to_launch {
        if let Some(entry) = app.apps.iter().find(|a| &a.exec == command).cloned() {
            // Add to recent before launching
            app.add_to_recent(entry.name.clone());
            crate::launch::launch_app(&entry, &app.config);
        } else {
            // Fallback: run directly (not tracked in recent)
            use std::process::Command;
            let _ = Command::new("sh").arg("-c").arg(command.clone()).spawn();
        }
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    search_position: &config::SearchPosition,
    cfg: &config::LauncherConfig,
) -> Result<()> {
    let mut last_input = Instant::now();

    loop {
        // Always draw current state
        terminal.draw(|f| ui::draw(f, app, search_position.clone(), cfg))?;

        // Calculate remaining time until timeout
        let poll_duration = if cfg.timeout > 0 {
            let elapsed = last_input.elapsed().as_secs();
            if elapsed >= cfg.timeout {
                break; // Timeout reached
            }
            // Poll for the remaining time or 100ms, whichever is shorter
            let remaining_secs = cfg.timeout - elapsed;
            Duration::from_millis((remaining_secs * 1000).min(100))
        } else {
            // No timeout configured, use default poll interval
            Duration::from_millis(100)
        };

        // Poll for events with calculated duration
        if crossterm::event::poll(poll_duration)? {
            match event::read()? {
                event::Event::Key(key) => {
                    last_input = Instant::now();
                    if events::handle_key(app, key)? {
                        break;
                    }
                }
                event::Event::Resize(_, _) => {
                    last_input = Instant::now();
                    warmup_icons(terminal, app, cfg)?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn warmup_icons<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    app: &App,
    cfg: &config::LauncherConfig,
) -> eyre::Result<()> {
    if app.categories.is_empty() {
        return Ok(());
    }

    let mut warmup_app = app.clone();

    if warmup_app.mode == Mode::DualPane {
        let old_focus = warmup_app.focus;

        warmup_app.mode = Mode::SinglePane;
        warmup_app.focus = Focus::Apps; // Single-pane leftmost focus
        terminal.draw(|f| ui::draw(f, &mut warmup_app, cfg.search_position.clone(), cfg))?;

        warmup_app.mode = Mode::DualPane;
        warmup_app.focus = Focus::Categories;
        terminal.draw(|f| ui::draw(f, &mut warmup_app, cfg.search_position.clone(), cfg))?;

        warmup_app.focus = old_focus;
    } else {
        warmup_app.focus = Focus::Apps;
        terminal.draw(|f| ui::draw(f, &mut warmup_app, cfg.search_position.clone(), cfg))?;
    }

    Ok(())
}
