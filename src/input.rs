use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    None,
    Quit,
    Cancel,
    NextTab,
    PrevTab,
    SwitchTab(usize),
    NewTab,
    CloseTab,
    ScrollUp(usize),
    ScrollDown(usize),
    ScrollToTop,
    ScrollToBottom,
    Reconnect,
    ToggleStatusBar,
    Help,
    SubmitLine,
    Backspace,
    InsertChar(char),
    CopySelection,
    PasteClipboard,
    HistoryUp,
    HistoryDown,
    AutoComplete,
    CtrlC,
    CtrlZ,
}

pub fn handle_event() -> anyhow::Result<AppAction> {
    if let Event::Key(key) = event::read()? {
        Ok(handle_key(key))
    } else {
        Ok(AppAction::None)
    }
}

fn handle_key(key: KeyEvent) -> AppAction {
    let mods = key.modifiers;

    match key.code {
        KeyCode::Char('q') if mods.contains(KeyModifiers::CONTROL) => AppAction::Quit,
        KeyCode::Esc => AppAction::Cancel,
        KeyCode::Tab if mods.contains(KeyModifiers::CONTROL) => {
            if mods.contains(KeyModifiers::SHIFT) {
                AppAction::PrevTab
            } else {
                AppAction::NextTab
            }
        }
        KeyCode::Char('1') if mods.contains(KeyModifiers::CONTROL) => AppAction::SwitchTab(0),
        KeyCode::Char('2') if mods.contains(KeyModifiers::CONTROL) => AppAction::SwitchTab(1),
        KeyCode::Char('3') if mods.contains(KeyModifiers::CONTROL) => AppAction::SwitchTab(2),
        KeyCode::Char('4') if mods.contains(KeyModifiers::CONTROL) => AppAction::SwitchTab(3),
        KeyCode::Char('5') if mods.contains(KeyModifiers::CONTROL) => AppAction::SwitchTab(4),
        KeyCode::Char('6') if mods.contains(KeyModifiers::CONTROL) => AppAction::SwitchTab(5),
        KeyCode::Char('7') if mods.contains(KeyModifiers::CONTROL) => AppAction::SwitchTab(6),
        KeyCode::Char('8') if mods.contains(KeyModifiers::CONTROL) => AppAction::SwitchTab(7),
        KeyCode::Char('9') if mods.contains(KeyModifiers::CONTROL) => AppAction::SwitchTab(8),
        KeyCode::Char('t') if mods.contains(KeyModifiers::CONTROL) => AppAction::NewTab,
        KeyCode::Char('w') if mods.contains(KeyModifiers::CONTROL) => AppAction::CloseTab,
        KeyCode::Up if mods.contains(KeyModifiers::SHIFT) => AppAction::ScrollUp(1),
        KeyCode::Down if mods.contains(KeyModifiers::SHIFT) => AppAction::ScrollDown(1),
        KeyCode::PageUp => AppAction::ScrollUp(20),
        KeyCode::PageDown => AppAction::ScrollDown(20),
        KeyCode::Home if mods.contains(KeyModifiers::SHIFT) => AppAction::ScrollToTop,
        KeyCode::End if mods.contains(KeyModifiers::SHIFT) => AppAction::ScrollToBottom,
        KeyCode::Up if mods.contains(KeyModifiers::CONTROL) => AppAction::ScrollUp(5),
        KeyCode::Down if mods.contains(KeyModifiers::CONTROL) => AppAction::ScrollDown(5),
        KeyCode::F(5) => AppAction::Reconnect,
        KeyCode::F(9) => AppAction::ToggleStatusBar,
        KeyCode::F(1) => AppAction::Help,
        // IMPORTANT: Ctrl+Shift+C must be checked BEFORE Ctrl+C
        KeyCode::Char('c')
            if mods.contains(KeyModifiers::CONTROL) && mods.contains(KeyModifiers::SHIFT) =>
        {
            AppAction::CopySelection
        }
        KeyCode::Char('c') if mods.contains(KeyModifiers::CONTROL) => AppAction::CtrlC,
        KeyCode::Char('z') if mods.contains(KeyModifiers::CONTROL) => AppAction::CtrlZ,
        KeyCode::Char('v') if mods.contains(KeyModifiers::CONTROL) => AppAction::PasteClipboard,
        KeyCode::Insert if mods.contains(KeyModifiers::SHIFT) => AppAction::PasteClipboard,
        KeyCode::Up if !mods.contains(KeyModifiers::SHIFT) => AppAction::HistoryUp,
        KeyCode::Down if !mods.contains(KeyModifiers::SHIFT) => AppAction::HistoryDown,
        KeyCode::Tab if !mods.contains(KeyModifiers::CONTROL) => AppAction::AutoComplete,
        KeyCode::Enter => AppAction::SubmitLine,
        KeyCode::Backspace => AppAction::Backspace,
        KeyCode::Char(c) => {
            if mods.contains(KeyModifiers::CONTROL) || mods.contains(KeyModifiers::ALT) {
                AppAction::None
            } else {
                AppAction::InsertChar(c)
            }
        }
        _ => AppAction::None,
    }
}

pub fn get_clipboard() -> Option<String> {
    arboard::Clipboard::new()
        .ok()
        .and_then(|mut cb| cb.get_text().ok().map(|s| s.to_string()))
}

pub fn set_clipboard(text: &str) -> bool {
    arboard::Clipboard::new()
        .ok()
        .and_then(|mut cb| cb.set_text(text.to_string()).ok())
        .is_some()
}
