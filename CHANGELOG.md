# Changelog

All notable changes to Catalyst will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-03-29

### Added

#### Context Engine
- **TokenCounter**: cl100k_base tokenizer for accurate token counting
- **TokenBudget**: per-model budget tracking (200k Claude, 128k GPT-4o)
- **FileCache**: LRU file cache with mtime validation and token counting
- **ContextEngine**: sliding window message builder with archive summaries
- **Budget warnings**: emits warning when context usage exceeds 80%
- **Output truncation**: head/tail preservation with omitted char count

#### Rich Tool System
- **GlobTool**: file pattern matching with mtime sorting and max results
- **GrepTool**: regex search with include filter, skips build directories
- **ListTool**: directory listing with human sizes, skip hidden/build dirs
- **Async tool trait**: all tools use async execute with output_limit()
- **Output truncation**: automatic truncation for large tool results
- **7 tools total**: read, write, edit, bash, glob, grep, list

#### Agent Intelligence
- **Max iteration guard**: stops at 25 iterations to prevent runaway
- **AgentState machine**: Idle/Planning/Executing/Verifying/Complete/Error/Cancelled
- **Error retry**: auto-retry on tool errors (configurable, default 2 retries)
- **CancellationToken**: Ctrl+C during streaming cancels, idle quits
- **State events**: StateChanged events for TUI status display

#### Project Awareness
- **Language detection**: Rust/TypeScript/JavaScript/Python/Go
- **File tree builder**: tree chars, depth/line limits, skip hidden/build
- **Git context detection**: branch, status, recent commits
- **Key file detection**: language-specific (Cargo.toml, tsconfig, go.mod, etc.)
- **Dynamic system prompt**: injects project context into system prompt

#### Session Persistence
- **SessionData**: save/load conversation history to JSON
- **Session directory**: ~/.config/catalyst/sessions/<id>.json
- **Session commands**: /sessions list, /session resume <id>, /session new

#### @file Reference Syntax
- **File expansion**: @path/to/file in messages auto-injects file content
- **File caching**: referenced files cached in ContextEngine
- **Truncation**: large files truncated with head/tail preservation

### Changed

#### Configuration
- Added: max_iterations, max_retries, auto_retry, project_awareness
- Added: working_window_size, max_tokens_per_request to config

#### TUI
- Status message display from agent state changes
- Ctrl+C cancellation during streaming vs idle
- Budget warning indicator in header
- Session management commands

### Technical Details

#### Test Coverage by Crate
- `catalyst-cli`: 15 tests
- `catalyst-core`: 97 tests (agent, context, events, project, session)
- `catalyst-llm`: 10 tests
- `catalyst-tools`: 35 tests (7 tools + registry)
- `catalyst-tui`: 44 tests (app, commands, key handling)
- **201 total tests** (up from 94 in v0.1.1)

#### New Dependencies
- tiktoken-rs 0.9 — token counting
- regex 1.11 — @file reference parsing
- dirs 6.0 — session directory resolution

## [0.1.1] - 2026-03-18

### Added

#### Testing
- **94 total tests** (up from 25) with comprehensive coverage
- CLI tests with clap `debug_assert()` validation
- Config module tests for serialization and CLI arg merging
- Command parsing tests for all slash commands
- TUI App state tests for event handling and state management
- Mock LLM provider for agent testing without real API calls
- ModelInfo and ProviderInfo tests
- Key handling tests for all input modes

#### Dependencies
- Added `async-trait` to catalyst-core for mock provider testing
- Added `serde_json` to catalyst-tui for test support

### Changed

#### CLI
- Added `--version` flag support
- Improved CLI validation with clap debug assertions

#### Code Quality
- Fixed all clippy warnings
- Added `#[allow(dead_code)]` for test-only helper functions

### Technical Details

#### Test Coverage by Crate
- `catalyst-cli`: 15 tests (CLI parsing, config)
- `catalyst-core`: 15 tests (agent, events, mock provider)
- `catalyst-llm`: 10 tests (type serialization, provider parsing)
- `catalyst-tools`: 10 tests (all 4 tools + registry)
- `catalyst-tui`: 44 tests (app state, commands, key handling)

## [0.1.0-alpha] - 2026-03-16

### Added

#### Core Features
- **Multi-provider support**: Anthropic Claude and OpenRouter integration
- **Terminal UI**: Beautiful, responsive interface built with ratatui
- **Streaming responses**: Real-time streaming with extended thinking visualization
- **Tool system**: 4 built-in tools (read, write, edit, bash)
- **Slash commands**: /help, /model, /clear, /config, /exit
- **Configuration**: TOML-based configuration with CLI overrides

#### Tools
- **read**: Read files with line numbers, offset, and limit support
- **write**: Create new files with automatic directory creation
- **edit**: Edit files with exact string matching and replace_all support
- **bash**: Execute shell commands with safety checks

#### Providers
- **Anthropic**: Full support for Claude models with extended thinking
  - claude-sonnet-4-20250514 (default)
  - claude-opus-4-20250514
  - claude-3-5-sonnet-20241022
  - claude-3-5-haiku-20241022
- **OpenRouter**: Multi-model support through OpenRouter
  - anthropic/claude-sonnet-4
  - anthropic/claude-opus-4
  - anthropic/claude-3.5-sonnet
  - openai/gpt-4o
  - google/gemini-pro-1.5

#### TUI Features
- Two input modes: Normal and Insert
- Real-time message streaming
- Tool execution feedback with status indicators
- Thinking process visualization
- System messages for notifications
- Token usage and cost tracking

#### Safety Features
- Path validation (files only accessible within working directory)
- Dangerous command blocking (rm -rf /, sudo rm, etc.)
- Tool execution timeouts
- Error handling and recovery

#### Architecture
- Multi-crate Rust workspace
  - catalyst-cli: CLI entry point and configuration
  - catalyst-core: Agent logic and event handling
  - catalyst-llm: LLM provider implementations
  - catalyst-tools: Tool implementations
  - catalyst-tui: Terminal UI

#### Testing
- 25 passing unit tests
- Tests for tools, LLM types, and agent events

#### Documentation
- README.md: Project overview and quick start
- USAGE.md: Detailed usage guide with examples
- CONTRIBUTING.md: Contribution guidelines

### Technical Details

#### Dependencies
- ratatui 0.29 - Terminal UI
- tokio 1.x - Async runtime
- reqwest 0.12 - HTTP client
- serde/serde_json - Serialization
- anyhow/thiserror - Error handling
- crossterm 0.28 - Terminal handling

#### Code Quality
- Zero clippy warnings
- All code formatted with rustfmt
- No unsafe code
- Comprehensive error handling with context

### Known Issues

#### High Priority
- No retry logic for network failures
- No streaming for very large files (>100MB)
- Some API errors may cause panics

#### Medium Priority
- No session persistence (conversations lost on exit)
- No pagination for long conversations
- No syntax highlighting for code blocks
- No file reference support (@file syntax)

#### Low Priority
- Only one color theme available
- No conversation export functionality
- No message search functionality

### Breaking Changes
None (initial release)

### Migration Guide
Not applicable (initial release)

### Contributors
- Initial development and release

---

## Release Notes Template

Use this template for future releases:

```markdown
## [X.Y.Z] - YYYY-MM-DD

### Added
- New features

### Changed
- Changes to existing features

### Deprecated
- Features to be removed in future releases

### Removed
- Features removed in this release

### Fixed
- Bug fixes

### Security
- Security improvements
```

---

## Version Naming Convention

- **alpha**: Early testing, unstable, may have breaking changes
- **beta**: Feature complete, needs testing, mostly stable
- **rc** (release candidate): Ready for release, final testing
- **stable**: Production-ready release

Example: `0.2.0-beta`, `0.3.0-rc.1`, `1.0.0`

---

## Support

For questions, issues, or contributions:
- GitHub Issues: https://github.com/akashrtd/catalyst/issues
- GitHub Discussions: https://github.com/akashrtd/catalyst/discussions
