use crate::config::Config;
use crate::input::AppAction;
use crate::tab::Tab;
use anyhow::Result;
use std::path::PathBuf;

pub struct App {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub config: Config,
    #[allow(dead_code)]
    pub config_path: PathBuf,
    pub running: bool,
    pub show_help: bool,
    pub show_status_bar: bool,
    pub connecting: bool,
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
                self.add_tab_interactive()?;
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
                self.connecting = true;
                let tab = self.active_tab_mut();
                if let Some(t) = tab {
                    let _ = t.reconnect();
                }
                self.connecting = false;
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
                        tab.history.push(line.clone());
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

    fn add_tab_interactive(&mut self) -> Result<()> {
        let name = format!("{}", self.tabs.len());
        let default_shell = self.config.general.default_shell.clone();
        let cols = self.config.general.default_cols;
        let rows = self.config.general.default_rows;

        let tc = crate::config::TerminalConfig {
            name: name.clone(),
            url: String::new(),
            token: String::new(),
            shell: Some(default_shell),
            cols: Some(cols),
            rows: Some(rows),
        };

        let scrollback_max = self.config.general.scrollback_lines;
        let tab = Tab::new(tc, scrollback_max);
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        Ok(())
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
