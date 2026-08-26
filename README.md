# agent-term-cli

A multi-tab TUI terminal emulator that connects to [agent-term](https://github.com/michelledupuis/agent-term) MCP servers.

- Multi-tab interface with auto-incremental naming
- Live screen polling with configurable refresh rate
- Command history with persistent storage and auto-completion
- Scrollback buffer with configurable line limit
- Clipboard integration (copy/paste)
- Keyboard-driven navigation (vim-like shortcuts)
- Cross-platform (Linux, macOS, Windows)

## Installation

### From source

```bash
cargo install agent-term-cli
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/michelledupuis/agent-term-cli/releases).

## Quick Start

1. **Start an agent-term server** (see [agent-term](https://github.com/michelledupuis/agent-term)):
   ```bash
   agent-term-server --port 8080 --token YOUR_SECRET
   ```

2. **Configure the CLI** — edit `~/.config/agent-term-cli/config.toml`:
   ```toml
   [[terminals]]
   name = "my-server"
   url = "http://localhost:8080/mcp"
   token = "YOUR_SECRET"
   shell = "bash"
   cols = 120
   rows = 40
   ```

3. **Run the CLI**:
   ```bash
   agent-term-cli
   ```

## Usage

```bash
agent-term-cli [OPTIONS]

Options:
  --config <PATH>   Path to config file
  --list            List configured terminals
  --help            Print help
```

## Keyboard Shortcuts

| Shortcut              | Action                  |
|-----------------------|-------------------------|
| `Ctrl+T`              | New terminal tab        |
| `Ctrl+W`              | Close current tab       |
| `Ctrl+Tab`            | Next tab                |
| `Ctrl+Shift+Tab`      | Previous tab            |
| `Ctrl+1..9`           | Switch to tab N         |
| `Ctrl+C`              | Send SIGINT             |
| `Ctrl+Z`              | Send SIGTSTP            |
| `Ctrl+R`              | Reverse history search  |
| `Tab`                 | Auto-complete           |
| `Up/Down`             | History navigation      |
| `Shift+Up/PageUp`     | Scroll up               |
| `Shift+Down/PageDown` | Scroll down             |
| `Ctrl+Shift+C`        | Copy to clipboard       |
| `Ctrl+V`              | Paste from clipboard    |
| `F5`                  | Reconnect               |
| `F9`                  | Toggle status bar       |
| `F1`                  | Help                    |
| `Ctrl+Q`              | Quit                    |

## Configuration

Config file location:
- **Linux**: `~/.config/agent-term-cli/config.toml`
- **macOS**: `~/Library/Application Support/agent-term-cli/config.toml`
- **Windows**: `%APPDATA%/agent-term-cli/config.toml`

### Config format

```toml
[general]
default_shell = "bash"       # or "cmd", "powershell", "zsh", etc.
default_cols = 120
default_rows = 40
poll_interval_ms = 100       # screen refresh interval (ms)
scrollback_lines = 10000     # scrollback buffer size

[[terminals]]
name = "my-server"
url = "http://localhost:8080/mcp"
token = "your-secret-token"
shell = "bash"
cols = 120
rows = 40
```

## History

Command history is stored per-tab at:
- **Linux**: `~/.config/agent-term-cli/history/<tab-name>.json`
- **macOS**: `~/Library/Application Support/agent-term-cli/history/<tab-name>.json`
- **Windows**: `%APPDATA%/agent-term-cli/history/<tab-name>.json`

## License

MIT
