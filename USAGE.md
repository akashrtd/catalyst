# Catalyst Usage Guide

This guide provides detailed instructions for using Catalyst effectively.

## Table of Contents

- [Getting Started](#getting-started)
- [Interface Overview](#interface-overview)
- [Slash Commands](#slash-commands)
- [Tool Usage](#tool-usage)
- [Configuration](#configuration)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Getting Started

### First Run

1. Set your API key as an environment variable:
   ```bash
   export ANTHROPIC_API_KEY=sk-ant-...
   ```

2. Navigate to your project directory:
   ```bash
   cd /path/to/your/project
   ```

3. Launch Catalyst:
   ```bash
   catalyst
   ```

### Initial Setup

On first run, Catalyst will:
- Display a welcome message
- Show your current provider and model
- Alert you if no API key is configured

## Interface Overview

### Layout

The TUI is divided into four sections:

```
┌─────────────────────────────────────────────┐
│ Header: Model, Tokens, Cost                 │
├─────────────────────────────────────────────┤
│                                             │
│ Messages: Conversation history              │
│           with AI responses, thinking,      │
│           and tool executions               │
│                                             │
├─────────────────────────────────────────────┤
│ Input: Your messages and commands           │
├─────────────────────────────────────────────┤
│ Footer: Keyboard shortcuts                  │
└─────────────────────────────────────────────┘
```

### Modes

Catalyst has two input modes:

1. **Normal Mode** (default)
   - Navigate and execute commands
   - Press `i` to enter insert mode
   - Press `q` or `Ctrl+C` to quit

2. **Insert Mode**
   - Type messages and commands
   - Press `Esc` to return to normal mode
   - Press `Enter` to send message

## Slash Commands

### /help, /h, /?

Display all available commands:

```
/help
```

Output:
```
┌─ Commands ─────────────────────────────┐
│ /help, /h, /?     Show this help      │
│ /model, /m <name> Switch model        │
│ /clear, /c        Clear conversation  │
│ /config, /cfg     Show config         │
│ /exit, /quit, /q  Exit Catalyst       │
└────────────────────────────────────────┘
```

### /model, /m

Switch between AI models or providers:

```
/model claude-opus-4-20250514
/m claude-3-5-sonnet-20241022
```

You can also switch providers:

```
/model openrouter
```

The TUI will prompt you to:
1. Select a provider (Anthropic or OpenRouter)
2. Enter your API key
3. Choose a model

### /clear, /c

Clear the conversation history:

```
/clear
```

This starts a fresh conversation while keeping your model and configuration.

### /config, /cfg

Display current configuration:

```
/config
```

Shows:
- Current provider
- Current model
- Working directory
- API key status (masked)

### /exit, /quit, /q

Exit Catalyst:

```
/exit
```

## Tool Usage

Catalyst automatically uses tools based on your requests. Here's how to phrase your requests:

### Reading Files

Instead of:
```
Please read the file src/main.rs
```

Try:
```
What's in src/main.rs?
Show me the contents of src/main.rs
Read src/main.rs and explain what it does
```

The AI will use the `read` tool to fetch the file contents.

### Writing Files

```
Create a new file called test.txt with "Hello World"
Write a Rust function to fibonacci and save it as fib.rs
```

### Editing Files

```
Change the variable name from x to count in main.rs
Update the error message in the validate function
Replace all occurrences of TODO with FIXME
```

### Running Commands

```
Run cargo test
Execute npm install
What's the output of git status?
```

### Multi-Step Operations

```
Read package.json, add a new dependency called "axios", and run npm install
```

The AI will:
1. Read the file
2. Edit it
3. Run the command

## Configuration

### Configuration File

Location: `~/.config/catalyst/config.toml`

Example configuration:

```toml
# Default model to use
model = "claude-sonnet-4-20250514"

# Provider: "anthropic" or "openrouter"
provider = "anthropic"

# API key (optional, can use environment variable instead)
api_key = "sk-ant-..."

# Default working directory
working_dir = "/Users/you/projects"
```

### Environment Variables

- `ANTHROPIC_API_KEY` - Anthropic API key
- `OPENROUTER_API_KEY` - OpenRouter API key

Environment variables take precedence over the config file.

### CLI Arguments

```bash
catalyst [OPTIONS]

Options:
  -d, --dir <DIR>                    Working directory
  -m, --model <MODEL>                Model to use
  -p, --provider <PROVIDER>          Provider (anthropic/openrouter)
      --api-key <API_KEY>            Anthropic API key
      --openrouter-api-key <KEY>     OpenRouter API key
```

CLI arguments override both environment variables and config file.

### Priority Order

1. CLI arguments (highest)
2. Environment variables
3. Configuration file
4. Defaults (lowest)

## Best Practices

### 1. Be Specific

**Good:**
```
Read src/parser.rs and add error handling for empty input
```

**Better:**
```
Read src/parser.rs and add error handling using anyhow for when the input string is empty. Return a descriptive error message.
```

### 2. Provide Context

```
I'm building a CLI tool that processes CSV files. 
Read src/main.rs and suggest improvements for error handling.
```

### 3. Break Down Complex Tasks

Instead of one large request:
```
Refactor the entire codebase to use async/await
```

Break it into steps:
```
1. First, identify which functions would benefit from async
2. Then convert the file I/O operations
3. Finally, update the network calls
```

### 4. Use Clear File Paths

```
Read src/utils/helpers.rs:45-60
Edit the `validate_email` function in src/validators/email.rs
```

### 5. Leverage Extended Thinking

Catalyst shows the AI's thinking process. Watch it to:
- Understand the AI's reasoning
- Catch potential issues early
- Learn better approaches

## Troubleshooting

### "No API key configured"

**Solution:**
```bash
export ANTHROPIC_API_KEY=your_key_here
```

Or use `/model` to set it interactively.

### "Path is outside working directory"

**Cause:** Trying to access files outside your project directory.

**Solution:** 
- Use relative paths
- Change working directory with `--dir`

### Tool Execution Timeout

**Cause:** Command taking too long.

**Solution:**
- Break into smaller operations
- Check for hanging processes
- Increase timeout in tool parameters

### Streaming Stops Mid-Response

**Cause:** Network issue or API rate limit.

**Solution:**
- Wait a moment and retry
- Check your API usage
- Try a different model

### TUI Display Issues

**Cause:** Terminal compatibility.

**Solution:**
- Ensure terminal supports 256 colors
- Try a different terminal emulator
- Check terminal size (minimum 80x24)

## Keyboard Shortcuts Reference

| Key | Mode | Action |
|-----|------|--------|
| `i` | Normal | Enter insert mode |
| `Esc` | Insert | Return to normal mode |
| `Enter` | Insert | Send message |
| `Ctrl+C` | Any | Exit Catalyst |
| `q` | Normal | Exit Catalyst |

## Tips and Tricks

### 1. Quick Model Switching

```
/m opus    # Switches to claude-opus-4
/m sonnet  # Switches to claude-sonnet-4
/m haiku   # Switches to claude-3-5-haiku
```

### 2. Clear and Restart

If a conversation gets confused:
```
/clear
```

Start fresh with the same model and configuration.

### 3. Check Context

Before asking for changes:
```
What files have we worked on?
What was the last change we made?
```

### 4. Verify Changes

After edits:
```
Read the file again to verify the changes
Run the tests to make sure nothing broke
```

### 5. Learn from Thinking

Watch the "Thinking:" sections to understand:
- Why the AI made certain choices
- What alternatives were considered
- Potential issues identified

## Getting Help

- **In-app:** Type `/help`
- **Issues:** https://github.com/catalyst/catalyst/issues
- **Documentation:** README.md and this file
