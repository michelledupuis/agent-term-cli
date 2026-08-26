use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::Path;

const DEFAULT_MAX_HISTORY: usize = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHistory {
    commands: VecDeque<String>,
    max_history: usize,
    #[serde(skip)]
    index: Option<usize>,
    #[serde(skip)]
    input_buffer: String,
}

impl CommandHistory {
    pub fn new(max_history: usize) -> Self {
        Self {
            commands: VecDeque::new(),
            max_history,
            index: None,
            input_buffer: String::new(),
        }
    }

    pub fn load(path: &Path) -> Self {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(history) = serde_json::from_str(&content) {
                return history;
            }
        }
        Self::new(DEFAULT_MAX_HISTORY)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn push(&mut self, command: String) {
        if command.is_empty() {
            return;
        }
        if self.commands.back().map(|s| s.as_str()) == Some(&command) {
            return;
        }
        self.commands.push_back(command);
        if self.commands.len() > self.max_history {
            self.commands.pop_front();
        }
    }

    pub fn up(&mut self) -> Option<&str> {
        if self.commands.is_empty() {
            return None;
        }
        match self.index {
            None => {
                let last = self.commands.len() - 1;
                self.index = Some(last);
                Some(&self.commands[last])
            }
            Some(0) => None,
            Some(i) => {
                self.index = Some(i - 1);
                Some(&self.commands[i - 1])
            }
        }
    }

    pub fn down(&mut self) -> Option<String> {
        match self.index {
            None => None,
            Some(i) => {
                if i + 1 >= self.commands.len() {
                    self.index = None;
                    Some(self.input_buffer.clone())
                } else {
                    self.index = Some(i + 1);
                    Some(self.commands[i + 1].clone())
                }
            }
        }
    }

    pub fn save_current_input(&mut self, input: &str) {
        self.input_buffer = input.to_string();
    }

    pub fn reset_index(&mut self) {
        self.index = None;
    }

    pub fn autocomplete(&self, prefix: &str) -> Vec<&str> {
        if prefix.is_empty() {
            return Vec::new();
        }
        self.commands
            .iter()
            .rev()
            .filter(|cmd| cmd.starts_with(prefix))
            .map(|s| s.as_str())
            .collect()
    }

    pub fn autocomplete_unique(&self, prefix: &str) -> Option<String> {
        let matches = self.autocomplete(prefix);
        if matches.is_empty() {
            None
        } else if matches.len() == 1 {
            Some(matches[0].to_string())
        } else {
            let first_chars: Vec<char> = matches[0].chars().collect();
            let mut common_len = first_chars.len();
            for m in &matches[1..] {
                let other_chars: Vec<char> = m.chars().collect();
                let mut i = 0;
                while i < common_len && i < other_chars.len() && first_chars[i] == other_chars[i] {
                    i += 1;
                }
                common_len = i;
            }
            Some(first_chars[..common_len].iter().collect())
        }
    }

    #[allow(dead_code)]
    pub fn all(&self) -> Vec<&str> {
        self.commands.iter().map(|s| s.as_str()).collect()
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_autocomplete() {
        let mut h = CommandHistory::new(100);
        h.push("dir".to_string());
        h.push("dir /s".to_string());
        h.push("echo hello".to_string());

        assert_eq!(h.len(), 3);
        assert_eq!(h.autocomplete("di").len(), 2);
        assert_eq!(h.autocomplete_unique("ech"), Some("echo hello".to_string()));
    }

    #[test]
    fn test_up_down() {
        let mut h = CommandHistory::new(100);
        h.push("first".to_string());
        h.push("second".to_string());
        h.push("third".to_string());

        assert_eq!(h.up(), Some("third"));
        assert_eq!(h.up(), Some("second"));
        assert_eq!(h.up(), Some("first"));
        assert_eq!(h.up(), None);

        assert_eq!(h.down(), Some("second".to_string()));
        assert_eq!(h.down(), Some("third".to_string()));
        assert_eq!(h.down(), Some(String::new()));
    }

    #[test]
    fn test_autocomplete_unique_unicode() {
        let mut h = CommandHistory::new(100);
        h.push("café".to_string());
        h.push("caféine".to_string());

        assert_eq!(h.autocomplete_unique("café"), Some("café".to_string()));
    }
}
