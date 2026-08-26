use crate::config::Config;
use crate::input::AppAction;
use crate::tab::Tab;
use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptMode {
    Url,
    Token,
    Shell,
}

pub struct App {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub config: Config,
    pub config_path: PathBuf,
    pub running: bool,
    pub show_help: bool,
    pub show_status_bar: bool,
    pub connecting: bool,
    pub prompt_mode: Option<PromptMode>,
    pub prompt_url: String,
    pub prompt_token: String,
    pub prompt_shell: String,
    pub prompt_cursor: usize,
}

impl App {
    pub fn new(config: Config, config_path: PathBuf) -> Self {
        let scrollback_max = config.general.scrollback_lines;
        let tabs: Vec<Tab> = config
            .terminals
            .iter()
            .map(|tc| Tab::new(tc.clone(), scrollback_max))
            .collect();

        Self {
            tabs,
            active_tab: 0,
            config,
            config_path,
            running: true,
            show_help: false,
            show_status_bar: true,
            connecting: false,
            prompt_mode: None,
            prompt_url: String::new(),
            prompt_token: String::new(),
            prompt_shell: String::new(),
            prompt_cursor: 0,
        }
    }

    #[allow(dead_code)]
    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab)
    }

    pub fn handle_action(&mut self, action: AppAction) -> Result<()> {
        if self.show_help {
            self.show_help = false;
            return Ok(());
        }

        // Handle prompt mode input
        if self.prompt_mode.is_some() {
            return self.handle_prompt_action(action);
        }

        match action {
            AppAction::Quit => {
                self.running = false;
            }
            AppAction::NextTab => {
                if !self.tabs.is_empty() {
                    self.active_tab = (self.active_tab + 1) % self.tabs.len();
                }
            }
            AppAction::PrevTab => {
                if !self.tabs.is_empty() {
                    self.active_tab = if self.active_tab == 0 {
                        self.tabs.len() - 1
                    } else {
                        self.active_tab - 1
                    };
                }
            }
            AppAction::SwitchTab(n) => {
                if n < self.tabs.len() {
                    self.active_tab = n;
                }
            }
            AppAction::NewTab => {
                self.start_prompt();
            }
            AppAction::CloseTab => {
                self.close_active_tab();
            }
            AppAction::ScrollUp(amount) => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.scrollback.scroll_up(amount);
                }
            }
            AppAction::ScrollDown(amount) => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.scrollback.scroll_down(amount);
                }
            }
            AppAction::ScrollToTop => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.scrollback.scroll_to_top();
                }
            }
            AppAction::ScrollToBottom => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.scrollback.scroll_to_bottom();
                }
            }
            AppAction::Reconnect => {
                let idx = self.active_tab;
                let should_reconnect = self.tabs.get(idx).is_some_and(|tab| {
                    tab.state == crate::tab::TabState::Disconnected && !tab.config.url.is_empty()
                });
                if should_reconnect {
                    self.connecting = true;
                    if let Some(tab) = self.active_tab_mut() {
                        let _ = tab.reconnect();
                    }
                    self.connecting = false;
                }
            }
            AppAction::ToggleStatusBar => {
                self.show_status_bar = !self.show_status_bar;
            }
            AppAction::Help => {
                self.show_help = true;
            }
            AppAction::SubmitLine => {
                if let Some(tab) = self.active_tab_mut() {
                    if tab.state == crate::tab::TabState::Active {
                        let line = tab.input_buffer.clone();
                        if !line.is_empty() {
                            tab.history.push(line.clone());
                        }
                        tab.history.reset_index();

                        let cmd = format!("{}\r", line);
                        let _ = tab.send_input(&cmd);

                        tab.input_buffer.clear();
                        tab.cursor_x = 0;
                    }
                }
            }
            AppAction::Backspace => {
                if let Some(tab) = self.active_tab_mut() {
                    if tab.input_buffer.pop().is_some() {
                        tab.cursor_x = tab.cursor_x.saturating_sub(1);
                    }
                }
            }
            AppAction::InsertChar(c) => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.input_buffer.push(c);
                    tab.cursor_x += 1;
                }
            }
            AppAction::InsertString(s) => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.input_buffer.push_str(&s);
                    tab.cursor_x += s.len();
                }
            }
            AppAction::HistoryUp => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.history.save_current_input(&tab.input_buffer);
                    if let Some(cmd) = tab.history.up() {
                        tab.input_buffer = cmd.to_string();
                        tab.cursor_x = tab.input_buffer.len();
                    }
                }
            }
            AppAction::HistoryDown => {
                if let Some(tab) = self.active_tab_mut() {
                    if let Some(cmd) = tab.history.down() {
                        tab.input_buffer = cmd;
                        tab.cursor_x = tab.input_buffer.len();
                    }
                }
            }
            AppAction::AutoComplete => {
                if let Some(tab) = self.active_tab_mut() {
                    let prefix = tab.input_buffer.clone();
                    if let Some(completed) = tab.history.autocomplete_unique(&prefix) {
                        tab.input_buffer = completed;
                        tab.cursor_x = tab.input_buffer.len();
                    }
                }
            }
            AppAction::CtrlC => {
                if let Some(tab) = self.active_tab_mut() {
                    if tab.state == crate::tab::TabState::Active {
                        let _ = tab.send_input("\x03");
                    }
                }
            }
            AppAction::CtrlZ => {
                if let Some(tab) = self.active_tab_mut() {
                    if tab.state == crate::tab::TabState::Active {
                        let _ = tab.send_input("\x1a");
                    }
                }
            }
            AppAction::CopySelection => {}
            AppAction::PasteClipboard => {
                if let Some(text) = crate::input::get_clipboard() {
                    if let Some(tab) = self.active_tab_mut() {
                        if tab.state == crate::tab::TabState::Active {
                            let cleaned = text.replace(['\r', '\n'], "\r");
                            let _ = tab.send_input(&cleaned);
                        }
                    }
                }
            }
            AppAction::ReverseSearch => {}
            AppAction::ToggleScrollback => {
                if let Some(tab) = self.active_tab_mut() {
                    if tab.scrollback.is_at_bottom() {
                        tab.scrollback.scroll_to_top();
                    } else {
                        tab.scrollback.scroll_to_bottom();
                    }
                }
            }
            AppAction::None => {}
        }
        Ok(())
    }

    fn handle_prompt_action(&mut self, action: AppAction) -> Result<()> {
        match action {
            AppAction::Quit => {
                self.prompt_mode = None;
                self.running = false;
            }
            AppAction::Help => {
                self.prompt_mode = None;
            }
            AppAction::SubmitLine => {
                self.advance_prompt();
            }
            AppAction::Backspace => {
                match self.prompt_mode.as_ref().unwrap() {
                    PromptMode::Url => {
                        self.prompt_url.pop();
                        self.prompt_cursor = self.prompt_url.len();
                    }
                    PromptMode::Token => {
                        self.prompt_token.pop();
                        self.prompt_cursor = self.prompt_token.len();
                    }
                    PromptMode::Shell => {
                        self.prompt_shell.pop();
                        self.prompt_cursor = self.prompt_shell.len();
                    }
                }
            }
            AppAction::InsertChar(c) => {
                match self.prompt_mode.as_ref().unwrap() {
                    PromptMode::Url => {
                        self.prompt_url.push(c);
                        self.prompt_cursor = self.prompt_url.len();
                    }
                    PromptMode::Token => {
                        self.prompt_token.push(c);
                        self.prompt_cursor = self.prompt_token.len();
                    }
                    PromptMode::Shell => {
                        self.prompt_shell.push(c);
                        self.prompt_cursor = self.prompt_shell.len();
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn start_prompt(&mut self) {
        self.prompt_mode = Some(PromptMode::Url);
        self.prompt_url.clear();
        self.prompt_token.clear();
        self.prompt_shell = self.config.general.default_shell.clone();
        self.prompt_cursor = 0;
    }

    fn advance_prompt(&mut self) {
        match self.prompt_mode.as_ref().unwrap() {
            PromptMode::Url => {
                if self.prompt_url.is_empty() {
                    return;
                }
                self.prompt_mode = Some(PromptMode::Token);
                self.prompt_cursor = 0;
            }
            PromptMode::Token => {
                self.prompt_mode = Some(PromptMode::Shell);
                self.prompt_cursor = self.prompt_shell.len();
            }
            PromptMode::Shell => {
                self.finish_prompt();
            }
        }
    }

    fn finish_prompt(&mut self) {
        let name = self.find_next_tab_name();
        let tc = crate::config::TerminalConfig {
            name,
            url: self.prompt_url.trim().to_string(),
            token: self.prompt_token.trim().to_string(),
            shell: if self.prompt_shell.is_empty() {
                None
            } else {
                Some(self.prompt_shell.clone())
            },
            cols: Some(self.config.general.default_cols),
            rows: Some(self.config.general.default_rows),
        };

        let scrollback_max = self.config.general.scrollback_lines;
        let mut tab = Tab::new(tc, scrollback_max);

        // Try to connect immediately
        let _ = tab.connect();

        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        self.prompt_mode = None;
    }

    fn find_next_tab_name(&self) -> String {
        let mut i = 1;
        loop {
            let name = format!("Tab {}", i);
            if !self.tabs.iter().any(|t| t.name == name) {
                return name;
            }
            i += 1;
        }
    }

    fn close_active_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.tabs.remove(self.active_tab);
        if self.tabs.is_empty() {
            self.running = false;
        } else if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
    }
}
