# Catalyst

A research-driven AI coding agent with a beautiful terminal user interface.

## Features

- **Multi-Provider Support**: Works with Anthropic Claude and OpenRouter
- **Terminal UI**: Beautiful, responsive interface built with ratatui
- **Streaming Responses**: Real-time streaming of AI responses with extended thinking
- **Rich Tool System**: 7 built-in tools — read, write, edit, bash, glob, grep, list
- **Context Engine**: Token budgeting, sliding window, file caching, output truncation
- **Project Awareness**: Auto-detects language, file tree, git context, key files
- **@file References**: Use `@path/to/file` in messages to inject file contents
- **Session Persistence**: Save and resume conversations with `/sessions`
- **Cancellation**: Ctrl+C during streaming cancels, during idle quits
- **Configurable**: TOML-based configuration with CLI overrides

## Installation

### From Source

```bash
git clone https://github.com/catalyst/catalyst.git
cd catalyst
cargo install --path catalyst-cli
```

### Prerequisites

- Rust 1.70 or later
- An API key from Anthropic or OpenRouter

## Quick Start

1. Set your API key:
```bash
export ANTHROPIC_API_KEY=your_key_here
# or for OpenRouter
export OPENROUTER_API_KEY=your_key_here
```

2. Run Catalyst:
```bash
catalyst
```

3. Start chatting! Type your message and press Enter.

## Usage

### Basic Commands

```bash
# Run in current directory
catalyst

# Specify working directory
catalyst --dir /path/to/project

# Choose a specific model
catalyst --model claude-sonnet-4-20250514

# Use OpenRouter
catalyst --provider openrouter --model anthropic/claude-3.5-sonnet
```

### Slash Commands

Inside the TUI, use these commands:

- `/help` or `/h` - Show available commands
- `/model <name>` or `/m <name>` - Switch AI model
- `/clear` or `/c` - Clear conversation history
- `/config` or `/cfg` - Show current configuration
- `/exit` or `/quit` or `/q` - Exit Catalyst

### Keyboard Shortcuts

- `i` - Enter insert mode (to type messages)
- `Esc` - Return to normal mode
- `Enter` - Send message (in insert mode)
- `Ctrl+C` - Exit Catalyst

## Available Models

### Anthropic
- `claude-sonnet-4-20250514` (default)
- `claude-opus-4-20250514`
- `claude-3-5-sonnet-20241022`
- `claude-3-5-haiku-20241022`

### OpenRouter
- `anthropic/claude-sonnet-4`
- `anthropic/claude-opus-4`
- `anthropic/claude-3.5-sonnet`
- `openai/gpt-4o`
- `google/gemini-pro-1.5`

## Tools

Catalyst has 7 built-in tools:

1. **read** - Read file contents with line numbers
   - Parameters: `path`, `offset` (optional), `limit` (optional)

2. **write** - Create new files
   - Parameters: `path`, `content`

3. **edit** - Edit existing files by replacing text
   - Parameters: `path`, `old_string`, `new_string`, `replace_all` (optional)

4. **bash** - Execute shell commands safely
   - Parameters: `command`, `timeout` (optional)

5. **glob** - Find files matching a pattern
   - Parameters: `pattern`, `max_results` (optional)

6. **grep** - Search file contents with regex
   - Parameters: `pattern`, `include` (optional)

7. **list** - List directory contents with metadata
   - Parameters: `path`

## Configuration

Catalyst looks for configuration in `~/.config/catalyst/config.toml`:

```toml
model = "claude-sonnet-4-20250514"
provider = "anthropic"
api_key = "your-api-key"
working_dir = "/path/to/default/project"
```

CLI arguments override configuration file settings.

## Development

### Building

```bash
cargo build --workspace
```

### Testing

```bash
cargo test --workspace
```

### Linting

```bash
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

### Running in Development

```bash
cargo run --package catalyst-cli
```

## Architecture

Catalyst is organized as a multi-crate Rust workspace:

- **catalyst-cli**: Command-line interface and configuration
- **catalyst-core**: Agent logic and event handling
- **catalyst-llm**: LLM provider clients (Anthropic, OpenRouter)
- **catalyst-tools**: Tool implementations (read, write, edit, bash)
- **catalyst-tui**: Terminal UI with ratatui

## Safety Features

- Path validation ensures files are only accessed within the working directory
- Dangerous bash commands are blocked (e.g., `rm -rf /`)
- All tools run in isolated contexts with timeouts
- Error handling prevents cascading failures

## License

MIT

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Support

- GitHub Issues: https://github.com/catalyst/catalyst/issues
- Documentation: See [USAGE.md](USAGE.md) for detailed usage instructions
