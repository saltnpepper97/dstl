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
use std::io;
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
    
    // Load launcher configuration
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
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Initialize app state based on config mode
    let mut app = App::new(single_pane_mode, start_mode, &cfg);
    warmup_icons(&mut terminal, &app, &cfg)?;

    // --- First-frame warmup for category icons ---
    if !app.categories.is_empty() {
        let old_mode = app.mode;
        let old_focus = app.focus;

        // Temporarily switch to dual-pane and focus categories
        app.mode = Mode::DualPane;
        app.focus = Focus::Categories;

        // Draw a hidden frame to "warm up" glyph rendering
        terminal.draw(|f| ui::draw(f, &mut app, cfg.search_position.clone(), &cfg))?;

        // Restore original mode/focus
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
    if let Some(command) = app.app_to_launch {
        if let Some(entry) = app.apps.iter().find(|a| a.exec == command) {
            crate::launch::launch_app(entry, &app.config);
        } else {
            // Fallback: run directly
            use std::process::Command;
            let _ = Command::new("sh").arg("-c").arg(command).spawn();
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
    loop {
        // Draw UI
        terminal.draw(|f| ui::draw(f, app, search_position.clone(), cfg))?;
        
        // Handle input
        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                event::Event::Key(key) => {
                    if events::handle_key(app, key)? {
                        break;
                    }
                }
                event::Event::Resize(_, _) => {
                    // Re-warmup icons after resize to fix glyph sizing
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

    // Clone the app so we don't mutate real state
    let mut warmup_app = app.clone();

    // If starting in DualPane, briefly switch to SinglePane and back
    if warmup_app.mode == Mode::DualPane {
        // Save original focus
        let old_focus = warmup_app.focus;

        // Switch to SinglePane
        warmup_app.mode = Mode::SinglePane;
        warmup_app.focus = Focus::Apps; // Single-pane leftmost focus
        terminal.draw(|f| ui::draw(f, &mut warmup_app, cfg.search_position.clone(), cfg))?;

        // Switch back to DualPane
        warmup_app.mode = Mode::DualPane;
        warmup_app.focus = Focus::Categories;
        terminal.draw(|f| ui::draw(f, &mut warmup_app, cfg.search_position.clone(), cfg))?;

        // Restore original focus
        warmup_app.focus = old_focus;
    } else {
        // SinglePane start: just draw once
        warmup_app.focus = Focus::Apps;
        terminal.draw(|f| ui::draw(f, &mut warmup_app, cfg.search_position.clone(), cfg))?;
    }

    Ok(())
}
