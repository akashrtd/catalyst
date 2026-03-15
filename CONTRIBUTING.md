# Contributing to Catalyst

Thank you for your interest in contributing to Catalyst! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Commit Guidelines](#commit-guidelines)
- [Pull Request Process](#pull-request-process)
- [Testing](#testing)
- [Documentation](#documentation)

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inspiring community for all. Please be respectful and constructive in all interactions.

### Expected Behavior

- Be respectful and inclusive
- Welcome newcomers
- Focus on what is best for the community
- Show empathy towards other community members

### Unacceptable Behavior

- Harassment or discrimination
- Trolling or insulting comments
- Public or private harassment
- Publishing others' private information

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Git
- A code editor (VS Code, IntelliJ IDEA, etc.)

### Initial Setup

1. Fork and clone the repository:
```bash
git clone https://github.com/YOUR_USERNAME/catalyst.git
cd catalyst
```

2. Install development dependencies:
```bash
# Rust should already be installed
cargo --version
```

3. Build the project:
```bash
cargo build --workspace
```

4. Run tests:
```bash
cargo test --workspace
```

5. Run clippy:
```bash
cargo clippy --all-targets -- -D warnings
```

### Project Structure

```
catalyst/
├── catalyst-cli/      # CLI entry point and configuration
├── catalyst-core/     # Agent logic and event handling
├── catalyst-llm/      # LLM provider implementations
├── catalyst-tools/    # Tool implementations
├── catalyst-tui/      # Terminal UI
├── Cargo.toml         # Workspace configuration
├── README.md          # Project overview
├── USAGE.md           # Detailed usage guide
└── CONTRIBUTING.md    # This file
```

## How to Contribute

### Reporting Bugs

Before submitting a bug report:

1. Check existing issues to avoid duplicates
2. Test with the latest version
3. Collect information:
   - Operating system and version
   - Rust version (`rustc --version`)
   - Catalyst version (`catalyst --version` or commit hash)
   - Steps to reproduce
   - Expected vs actual behavior
   - Logs or screenshots

Submit issues at: https://github.com/catalyst/catalyst/issues

### Suggesting Enhancements

Enhancement suggestions are welcome! Please:

1. Use a clear and descriptive title
2. Provide a detailed description of the enhancement
3. Explain why this enhancement would be useful
4. List any alternatives you've considered
5. Include examples if applicable

### Pull Requests

We welcome pull requests! Here's how to submit one:

1. Create a feature branch from `main`
2. Make your changes
3. Add or update tests
4. Update documentation
5. Submit a pull request

## Development Workflow

### 1. Create a Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/your-bug-fix
```

Use descriptive branch names:
- `feature/add-new-tool`
- `fix/memory-leak`
- `docs/update-readme`
- `refactor/improve-error-handling`

### 2. Make Changes

- Write clean, readable code
- Follow existing code style
- Add comments for complex logic
- Keep functions focused and small

### 3. Test Your Changes

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p catalyst-tools

# Run a specific test
cargo test test_read_tool
```

### 4. Check Code Quality

```bash
# Run clippy
cargo clippy --all-targets -- -D warnings

# Format code
cargo fmt

# Check formatting
cargo fmt --check
```

### 5. Commit Your Changes

See [Commit Guidelines](#commit-guidelines) below.

### 6. Push and Create PR

```bash
git push origin feature/your-feature-name
```

Then create a pull request on GitHub.

## Coding Standards

### Rust Style

- Follow standard Rust conventions
- Use `cargo fmt` for formatting
- Resolve all clippy warnings
- Prefer `Result<T>` over panics
- Use meaningful variable and function names

### Code Organization

- One module per file
- Group related functionality
- Keep modules focused
- Use `pub` only when necessary

### Error Handling

```rust
// Good
use anyhow::{Context, Result};

pub fn read_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))
}

// Avoid
pub fn read_file(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap() // Can panic!
}
```

### Documentation

- Document public APIs with doc comments
- Include examples in documentation
- Update README.md for user-facing changes
- Update USAGE.md for usage changes

```rust
/// Reads a file from the filesystem.
///
/// # Arguments
///
/// * `path` - The path to the file to read
///
/// # Returns
///
/// The file contents as a string with line numbers
///
/// # Errors
///
/// Returns an error if the file doesn't exist or can't be read
///
/// # Example
///
/// ```
/// let content = read_file("src/main.rs")?;
/// println!("{}", content);
/// ```
pub fn read_file(path: &str) -> Result<String> {
    // ...
}
```

### Testing

- Write unit tests for all public functions
- Use descriptive test names
- Test edge cases and error conditions
- Keep tests independent

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_file_returns_content() {
        // Setup
        let content = "test content";
        
        // Execute
        let result = read_file("test.txt");
        
        // Assert
        assert!(result.is_ok());
        assert!(result.unwrap().contains(content));
    }

    #[test]
    fn test_read_file_fails_for_nonexistent() {
        let result = read_file("nonexistent.txt");
        assert!(result.is_err());
    }
}
```

## Commit Guidelines

### Commit Message Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

### Examples

```
feat(tools): add grep tool for file searching

Add a new grep tool that allows searching file contents using
regular expressions. This enables more efficient code searches.

Closes #123
```

```
fix(llm): handle rate limiting in anthropic client

Add retry logic with exponential backoff for rate limit errors.
This prevents crashes when hitting API limits.

Fixes #456
```

```
docs(readme): update installation instructions

Add installation steps for macOS and Linux.
```

### Tips

- Keep commits atomic (one logical change per commit)
- Write clear, descriptive commit messages
- Reference issues and pull requests
- Use present tense ("add feature" not "added feature")

## Pull Request Process

### Before Submitting

1. Ensure all tests pass
2. Run clippy without warnings
3. Format code with `cargo fmt`
4. Update documentation
5. Add tests for new functionality

### PR Template

```markdown
## Description

Brief description of changes

## Type of Change

- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing

- [ ] Tests pass locally
- [ ] New tests added
- [ ] Manual testing performed

## Checklist

- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] No new warnings
- [ ] Tests added/updated
```

### Review Process

1. Automated checks must pass
2. At least one maintainer review required
3. Address all review comments
4. Squash commits if requested

### After Merge

- Delete your feature branch
- Update your local main branch
- Celebrate! 🎉

## Testing

### Running Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p catalyst-core

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture
```

### Writing Tests

- Place tests in the same file as the code
- Use `#[cfg(test)]` module
- Test behavior, not implementation
- Use descriptive names

### Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_works() {
        // Arrange
        let input = "test";
        
        // Act
        let result = process(input);
        
        // Assert
        assert_eq!(result, expected);
    }
}
```

## Documentation

### Types of Documentation

1. **Code Documentation**
   - Doc comments for public APIs
   - Inline comments for complex logic

2. **User Documentation**
   - README.md - Overview and quick start
   - USAGE.md - Detailed usage instructions

3. **Contributor Documentation**
   - CONTRIBUTING.md - This file
   - Code comments and architecture docs

### Updating Documentation

When making changes:

1. Update doc comments for changed APIs
2. Update README.md for user-facing changes
3. Update USAGE.md for behavior changes
4. Add examples for new features

## Getting Help

- **Issues:** https://github.com/catalyst/catalyst/issues
- **Discussions:** https://github.com/catalyst/catalyst/discussions
- **Email:** [maintainer email]

## Recognition

Contributors are recognized in:
- Git commit history
- Release notes
- Contributors file (if created)

Thank you for contributing to Catalyst! 🙏
