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

    /// List configured terminals
    #[arg(long)]
    list: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Set panic hook to restore terminal state on crash
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));

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
            println!("[{}] {} — {} ({})", i + 1, t.name, t.url, t.shell.as_deref().unwrap_or("bash"));
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

    // Auto-connect to all terminals on startup (non-blocking)
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
                if let Err(e) = tab.poll_screen() {
                    eprintln!("Poll error: {}", e);
                }
            }
            last_poll = std::time::Instant::now();
        }

        // Draw UI
        terminal.draw(|frame| {
            ui::draw(frame, &app);
            if app.show_help && app.prompt_mode.is_none() {
                ui::draw_help_overlay(frame);
            }
        })?;

        // Handle input (process all queued events to prevent input lag)
        while crossterm::event::poll(Duration::ZERO)? {
            let action = input::handle_event()?;
            app.handle_action(action)?;
        }
    }

    // Cleanup - end all sessions
    for tab in app.tabs.iter_mut() {
        tab.end_session();
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
