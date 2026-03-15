# Changelog

All notable changes to Catalyst will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Session persistence (planned for v0.2.0)
- File references with @file syntax (planned for v0.2.0)
- Enhanced tool system with glob, grep, list (planned for v0.2.0)

### Changed
- Improved error handling (planned for v0.2.0)
- Better API key security (planned for v0.2.0)

### Fixed
- Various bug fixes and improvements (planned for v0.2.0)

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
