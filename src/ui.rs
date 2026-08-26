use crate::app::App;
use crate::tab::TabState;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Tabs};
use ratatui::Frame;

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(1),   // Terminal content
            Constraint::Length(3), // Status bar
        ])
        .split(frame.area());

    draw_tab_bar(frame, app, chunks[0]);
    draw_terminal(frame, app, chunks[1]);
    draw_status_bar(frame, app, chunks[2]);
}

fn draw_tab_bar(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = app
        .tabs
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            let prefix = if i == app.active_tab { "* " } else { "  " };
            let state_icon = match tab.state {
                TabState::Active => "",
                TabState::Disconnected => " ⚠",
                TabState::Connecting | TabState::Starting => " ...",
                TabState::Reconnecting => " ↻",
            };
            let scrollback_badge = if !tab.scrollback.is_at_bottom() {
                " ↑"
            } else {
                ""
            };
            Line::from(vec![
                Span::styled(
                    format!("{}[{}: {}{}{}]", prefix, tab.name, tab.config.shell.as_deref().unwrap_or("?"), state_icon, scrollback_badge),
                    if i == app.active_tab {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
            ])
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("agent-term-cli"))
        .select(app.active_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(tabs, area);
}

fn draw_terminal(frame: &mut Frame, app: &App, area: Rect) {
    let tab = match app.tabs.get(app.active_tab) {
        Some(t) => t,
        None => {
            let empty = Paragraph::new("No terminals. Press Ctrl+T to create one.");
            frame.render_widget(empty, area);
            return;
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", tab.name))
        .border_style(match tab.state {
            TabState::Active => Style::default().fg(Color::Green),
            TabState::Disconnected => Style::default().fg(Color::Red),
            TabState::Connecting | TabState::Starting | TabState::Reconnecting => {
                Style::default().fg(Color::Yellow)
            }
        });

    let inner = block.inner(area);
    frame.render_widget(block, area);

    match tab.state {
        TabState::Disconnected => {
            let msg = if app.connecting {
                "Connecting to server..."
            } else {
                "Disconnected. Press F5 to reconnect."
            };
            let para = Paragraph::new(msg).style(Style::default().fg(Color::Red));
            frame.render_widget(para, inner);
        }
        TabState::Connecting | TabState::Starting => {
            let msg = format!("Connecting to {}...", tab.config.url);
            let para = Paragraph::new(msg).style(Style::default().fg(Color::Yellow));
            frame.render_widget(para, inner);
        }
        TabState::Reconnecting => {
            let msg = "Reconnecting...";
            let para = Paragraph::new(msg).style(Style::default().fg(Color::Yellow));
            frame.render_widget(para, inner);
        }
        TabState::Active => {
            draw_screen(frame, app, tab, inner);
        }
    }
}

fn draw_screen(frame: &mut Frame, _app: &App, tab: &crate::tab::Tab, area: Rect) {
    let height = area.height as usize;
    let width = area.width as usize;

    // Get visible lines from scrollback
    let visible_lines = tab.scrollback.visible_lines(height);

    let mut lines: Vec<Line> = Vec::new();

    for line_text in &visible_lines {
        // Truncate to terminal width
        let display: String = line_text.chars().take(width).collect();
        lines.push(Line::from(Span::styled(
            display,
            Style::default().fg(Color::White),
        )));
    }

    // Add input line at bottom if we're at live view and have input
    if tab.scrollback.is_at_bottom() && !tab.input_buffer.is_empty() {
        let input_display: String = tab.input_buffer.chars().take(width).collect();
        lines.push(Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Green)),
            Span::styled(input_display, Style::default().fg(Color::White)),
        ]));
    }

    // Pad remaining lines
    while lines.len() < height {
        lines.push(Line::from(""));
    }

    let para = Paragraph::new(lines.clone());

    frame.render_widget(para, area);

    // Render cursor at input position when at live view
    if tab.scrollback.is_at_bottom() {
        let cursor_x = (tab.cursor_x + 2).min(width) as u16; // +2 for "> " prefix
        let cursor_y = (lines.len().saturating_sub(1)) as u16;
        frame.set_cursor_position((area.x + cursor_x, area.y + cursor_y));
    }
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    if !app.show_status_bar {
        return;
    }

    let tab = match app.tabs.get(app.active_tab) {
        Some(t) => t,
        None => return,
    };

    let (scroll_pos, scroll_total) = tab.scrollback.scroll_position();
    let scroll_info = if scroll_pos > 0 {
        format!(" 📜 {}/{}", scroll_total - scroll_pos, scroll_total)
    } else {
        String::new()
    };

    let status_left = format!(
        " Tab {} | {} | {} ",
        tab.name,
        tab.config.shell.as_deref().unwrap_or("?"),
        match tab.state {
            TabState::Active => "Connected",
            TabState::Disconnected => "Disconnected",
            TabState::Connecting => "Connecting...",
            TabState::Starting => "Starting...",
            TabState::Reconnecting => "Reconnecting...",
        }
    );

    let status_right = format!(
        "{}x{} | History: {}{} ",
        tab.cols(),
        tab.rows(),
        tab.history.len(),
        scroll_info
    );

    let line = Line::from(vec![
        Span::styled(&status_left, Style::default().fg(Color::White)),
        Span::raw(" ".repeat(area.width as usize - status_left.len() - status_right.len())),
        Span::styled(&status_right, Style::default().fg(Color::DarkGray)),
    ]);

    let para = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(para, area);
}

pub fn draw_help_overlay(frame: &mut Frame) {
    let area = frame.area();
    let popup_area = centered_rect(60, 50, area);
    frame.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(Span::styled(
            "agent-term-cli — Keyboard Shortcuts",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Ctrl+T       ", Style::default().fg(Color::Yellow)),
            Span::raw("New terminal"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+W       ", Style::default().fg(Color::Yellow)),
            Span::raw("Close terminal"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+Tab     ", Style::default().fg(Color::Yellow)),
            Span::raw("Next tab"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+1..9    ", Style::default().fg(Color::Yellow)),
            Span::raw("Switch to tab N"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+C       ", Style::default().fg(Color::Yellow)),
            Span::raw("Send SIGINT"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+Z       ", Style::default().fg(Color::Yellow)),
            Span::raw("Send SIGTSTP"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+R       ", Style::default().fg(Color::Yellow)),
            Span::raw("Reverse history search"),
        ]),
        Line::from(vec![
            Span::styled("  Tab          ", Style::default().fg(Color::Yellow)),
            Span::raw("Auto-complete from history"),
        ]),
        Line::from(vec![
            Span::styled("  Up/Down      ", Style::default().fg(Color::Yellow)),
            Span::raw("History navigation"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+Up/PgUp", Style::default().fg(Color::Yellow)),
            Span::raw("Scroll up"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+Dn/PgDn", Style::default().fg(Color::Yellow)),
            Span::raw("Scroll down"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+Shift+C ", Style::default().fg(Color::Yellow)),
            Span::raw("Copy to clipboard"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+V       ", Style::default().fg(Color::Yellow)),
            Span::raw("Paste from clipboard"),
        ]),
        Line::from(vec![
            Span::styled("  F5           ", Style::default().fg(Color::Yellow)),
            Span::raw("Reconnect"),
        ]),
        Line::from(vec![
            Span::styled("  F9           ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle status bar"),
        ]),
        Line::from(vec![
            Span::styled("  F1           ", Style::default().fg(Color::Yellow)),
            Span::raw("Show this help"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+Q       ", Style::default().fg(Color::Yellow)),
            Span::raw("Quit"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Block::default()
        .title(" Help (F1) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let para = Paragraph::new(help_text).block(block);
    frame.render_widget(para, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
