use crate::client::{McpClient, ScreenMetadata, ScreenOutput};
use crate::config::TerminalConfig;
use crate::history::CommandHistory;
use anyhow::Result;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabState {
    Disconnected,
    Connecting,
    Starting,
    Active,
}

pub struct Tab {
    pub name: String,
    pub config: TerminalConfig,
    pub state: TabState,
    pub client: McpClient,
    pub session_id: Option<String>,
    pub screen_buffer: Vec<String>,
    pub screen_metadata: Option<ScreenMetadata>,
    pub scrollback: Scrollback,
    pub history: CommandHistory,
    pub input_buffer: String,
    pub cursor_x: usize,
    pub last_screen: Option<String>,
}

pub struct Scrollback {
    pub lines: VecDeque<String>,
    pub max_lines: usize,
    pub cursor: usize,
}

impl Scrollback {
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(max_lines.min(1000)),
            max_lines,
            cursor: 0,
        }
    }

    pub fn append_lines(&mut self, new_lines: Vec<String>) {
        for line in new_lines {
            self.lines.push_back(line);
        }
        while self.lines.len() > self.max_lines {
            self.lines.pop_front();
        }
    }

    pub fn is_at_bottom(&self) -> bool {
        self.cursor == 0
    }

    pub fn scroll_up(&mut self, amount: usize) {
        let max_scroll = self.lines.len();
        self.cursor = self.cursor.saturating_add(amount).min(max_scroll);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.cursor = self.cursor.saturating_sub(amount);
    }

    pub fn scroll_to_top(&mut self) {
        self.cursor = self.lines.len();
    }

    pub fn scroll_to_bottom(&mut self) {
        self.cursor = 0;
    }

    pub fn visible_lines(&self, viewport_height: usize) -> Vec<&str> {
        if self.cursor == 0 {
            let start = self.lines.len().saturating_sub(viewport_height);
            self.lines.range(start..).map(|s| s.as_str()).collect()
        } else {
            let start = self.lines.len().saturating_sub(self.cursor + viewport_height);
            let end = self.lines.len().saturating_sub(self.cursor);
            self.lines.range(start..end).map(|s| s.as_str()).collect()
        }
    }

    pub fn scroll_position(&self) -> (usize, usize) {
        (self.cursor, self.lines.len())
    }
}

impl Tab {
    pub fn new(config: TerminalConfig, scrollback_max: usize) -> Self {
        let client = McpClient::new(&config.url, &config.token);
        let history_path = crate::config::Config::history_path(&config.name);
        let history = CommandHistory::load(&history_path);

        Self {
            name: config.name.clone(),
            config,
            state: TabState::Disconnected,
            client,
            session_id: None,
            screen_buffer: Vec::new(),
            screen_metadata: None,
            scrollback: Scrollback::new(scrollback_max),
            history,
            input_buffer: String::new(),
            cursor_x: 0,
            last_screen: None,
        }
    }

    pub fn connect(&mut self) -> Result<()> {
        if self.config.url.is_empty() {
            self.state = TabState::Disconnected;
            return Err(anyhow::anyhow!("No URL configured"));
        }

        self.state = TabState::Connecting;

        if !self.health_check() {
            self.state = TabState::Disconnected;
            return Err(anyhow::anyhow!("Server unreachable"));
        }

        self.state = TabState::Starting;
        self.client.initialize()?;

        let shell = self.config.shell.as_deref().unwrap_or("bash");
        let cols = self.config.cols.unwrap_or(120);
        let rows = self.config.rows.unwrap_or(40);

        let result = self.client.start_session(shell, cols, rows)?;
        self.session_id = Some(result.session_id.clone());
        self.state = TabState::Active;
        Ok(())
    }

    pub fn health_check(&self) -> bool {
        self.client.health_check()
    }

    pub fn send_input(&mut self, input: &str) -> Result<()> {
        if let Some(ref sid) = self.session_id {
            self.client.send_input(sid, input)?;
        }
        Ok(())
    }

    pub fn poll_screen(&mut self) -> Result<()> {
        if self.state != TabState::Active {
            return Ok(());
        }

        let session_id = match &self.session_id {
            Some(id) => id.clone(),
            None => return Ok(()),
        };

        match self.client.read_screen(&session_id) {
            Ok(output) => {
                self.update_screen(output);
                Ok(())
            }
            Err(e) => {
                if e.to_string().contains("Session not found")
                    || e.to_string().contains("broken pipe")
                {
                    self.state = TabState::Disconnected;
                    self.session_id = None;
                }
                Err(e)
            }
        }
    }

    fn update_screen(&mut self, output: ScreenOutput) {
        let lines: Vec<String> = output.output.lines().map(String::from).collect();

        // Only update cursor_x from metadata on first screen, not during user typing
        if self.last_screen.is_none() {
            self.screen_metadata = output.metadata;
            self.cursor_x = self
                .screen_metadata
                .as_ref()
                .map(|m| m.cursor_x)
                .unwrap_or(0);
        } else {
            // Update metadata but don't overwrite cursor_x when user is typing
            self.screen_metadata = output.metadata;
        }

        // Diff with last screen to find new lines for scrollback
        let screen_changed = self.last_screen.as_ref() != Some(&output.output);
        if screen_changed {
            if let Some(ref last) = self.last_screen {
                let old_lines: Vec<&str> = last.lines().collect();
                let new_lines: Vec<&str> = output.output.lines().collect();
                if new_lines.len() > old_lines.len() {
                    let appended: Vec<String> = new_lines[old_lines.len()..]
                        .iter()
                        .map(|s| s.to_string())
                        .collect();
                    self.scrollback.append_lines(appended);
                } else {
                    // Screen changed completely — replace scrollback tail
                    let replace_count = old_lines.len().min(self.scrollback.lines.len());
                    if replace_count > 0 {
                        let drain_start = self.scrollback.lines.len() - replace_count;
                        self.scrollback.lines.drain(drain_start..);
                    }
                    self.scrollback
                        .append_lines(new_lines.iter().map(|s| s.to_string()).collect());
                }
            } else {
                // First screen — add all lines to scrollback
                self.scrollback.append_lines(lines.clone());
            }
        }
        self.last_screen = Some(output.output);
        self.screen_buffer = lines;
    }

    pub fn end_session(&mut self) {
        if let Some(ref sid) = self.session_id {
            let _ = self.client.end_session(sid);
        }
        self.session_id = None;
        self.state = TabState::Disconnected;
        self.screen_buffer.clear();
        self.last_screen = None;
        self.input_buffer.clear();
        self.cursor_x = 0;

        let history_path = crate::config::Config::history_path(&self.name);
        let _ = self.history.save(&history_path);
    }

    pub fn disconnect(&mut self) {
        self.end_session();
    }

    pub fn reconnect(&mut self) -> Result<()> {
        self.disconnect();
        self.connect()
    }

    pub fn cols(&self) -> u16 {
        self.config.cols.unwrap_or(120)
    }

    pub fn rows(&self) -> u16 {
        self.config.rows.unwrap_or(40)
    }

    #[allow(dead_code)]
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.config.cols = Some(cols);
        self.config.rows = Some(rows);
        if let Some(ref sid) = self.session_id {
            self.client.resize(sid, cols, rows)?;
        }
        Ok(())
    }
}
