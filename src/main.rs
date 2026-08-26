mod app;
mod client;
mod config;
mod history;
mod input;
mod tab;
mod ui;

use anyhow::Result;
use clap::Parser;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "agent-term-cli", version, about = "Multi-tab terminal emulator for agent-term MCP servers")]
struct Args {
    /// Path to config file
    #[arg(long)]
    config: Option<String>,

    /// Add a new terminal interactively
    #[arg(long)]
    add: bool,

    /// List configured terminals
    #[arg(long)]
    list: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let config_path = match args.config {
        Some(p) => std::path::PathBuf::from(p),
        None => config::Config::default_path(),
    };

    let config = if config_path.exists() {
        config::Config::load(&config_path)?
    } else {
        let cfg = config::Config::default();
        cfg.save(&config_path)?;
        cfg
    };

    if args.list {
        for (i, t) in config.terminals.iter().enumerate() {
            println!("[{}] {} — {} ({})", i, t.name, t.url, t.shell.as_deref().unwrap_or("bash"));
        }
        return Ok(());
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new(config, config_path);

    // Auto-connect to all terminals on startup
    for tab in app.tabs.iter_mut() {
        let _ = tab.connect();
    }

    let poll_interval = Duration::from_millis(app.config.general.poll_interval_ms);
    let mut last_poll = std::time::Instant::now();

    // Main loop
    while app.running {
        // Poll screen from active tab periodically
        if last_poll.elapsed() >= poll_interval {
            if let Some(tab) = app.active_tab_mut() {
                let _ = tab.poll_screen();
            }
            last_poll = std::time::Instant::now();
        }

        // Draw UI
        terminal.draw(|frame| {
            ui::draw(frame, &app);
            if app.show_help {
                ui::draw_help_overlay(frame);
            }
        })?;

        // Handle input (with timeout so we don't block polling)
        if crossterm::event::poll(Duration::from_millis(10))? {
            let action = input::handle_event()?;
            app.handle_action(action)?;
        }
    }

    // Cleanup
    for tab in app.tabs.iter_mut() {
        tab.end_session();
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
