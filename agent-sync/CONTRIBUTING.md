# Contributing to Agent Sync

Thank you for your interest in contributing to Agent Sync! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Environment](#development-environment)
- [Making Contributions](#making-contributions)
- [Coding Standards](#coding-standards)
- [Testing Guidelines](#testing-guidelines)
- [Documentation Standards](#documentation-standards)
- [Pull Request Process](#pull-request-process)
- [Release Process](#release-process)

## Code of Conduct

We are committed to providing a welcoming and inclusive environment. Please read and follow our Code of Conduct:

- Be respectful and inclusive
- Welcome newcomers and help them get started
- Focus on constructive criticism
- Respect differing viewpoints and experiences
- Accept responsibility and apologize when making mistakes

## Getting Started

### Prerequisites

1. **Rust Toolchain** (1.75+)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup update stable
   ```

2. **Redis Server** (6.0+)
   ```bash
   # Windows (with WSL)
   wsl -e redis-server

   # Linux
   sudo apt-get install redis-server

   # Docker
   docker run -d -p 6379:6379 redis:latest
   ```

3. **Development Tools**
   ```bash
   # Required tools
   cargo install cargo-edit cargo-watch cargo-audit cargo-llvm-cov

   # Formatting and linting
   rustup component add rustfmt clippy

   # Documentation
   cargo install cargo-doc
   ```

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/redis-windows.git
   cd redis-windows/agent-sync
   ```

3. Add upstream remote:
   ```bash
   git remote add upstream https://github.com/anthropic/redis-windows.git
   ```

4. Create a feature branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Environment

### IDE Setup

#### Visual Studio Code

Install recommended extensions:
```json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "vadimcn.vscode-lldb",
    "serayuzgur.crates",
    "tamasfe.even-better-toml"
  ]
}
```

Settings for `.vscode/settings.json`:
```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy",
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

#### IntelliJ IDEA / CLion

- Install the Rust plugin
- Enable `cargo fmt` on save
- Configure Clippy as the external linter

### Build Configuration

The project uses multiple build profiles optimized for different scenarios:

```toml
# Development (fast compilation)
cargo build

# Development with optimizations
cargo build --profile dev-fast

# Release (maximum optimization)
cargo build --release

# Testing
cargo build --profile test

# Benchmarking
cargo build --profile bench
```

### Environment Variables

Create a `.env` file for local development:
```bash
# Redis Configuration
REDIS_URL=redis://localhost:6379
REDIS_PASSWORD=
REDIS_DB=0

# Logging
RUST_LOG=agent_sync=debug,redis=info
RUST_BACKTRACE=1

# Testing
TEST_REDIS_URL=redis://localhost:6380
TEST_TIMEOUT_SECS=30
```

## Making Contributions

### Types of Contributions

1. **Bug Fixes**: Fix issues reported in GitHub Issues
2. **Features**: Add new functionality
3. **Performance**: Optimize existing code
4. **Documentation**: Improve docs, examples, or comments
5. **Tests**: Add missing tests or improve coverage
6. **Refactoring**: Improve code structure and maintainability

### Contribution Workflow

1. **Check existing issues** or create a new one
2. **Discuss** your proposed changes
3. **Write code** following our standards
4. **Test** your changes thoroughly
5. **Document** new features or changes
6. **Submit** a pull request

### Creating Issues

When creating an issue, please include:

- **Clear title** describing the problem/feature
- **Description** with context and motivation
- **Steps to reproduce** (for bugs)
- **Expected behavior** vs actual behavior
- **Environment details** (OS, Rust version, Redis version)
- **Code examples** or error messages

Example issue template:
```markdown
## Description
Brief description of the issue

## Steps to Reproduce
1. Step one
2. Step two
3. ...

## Expected Behavior
What should happen

## Actual Behavior
What actually happens

## Environment
- OS: Windows 11 / Ubuntu 22.04 / macOS 14
- Rust: 1.75.0
- Redis: 7.0.5
- Agent-Sync: 0.1.0
```

## Coding Standards

### Rust Style Guide

Follow the official [Rust Style Guide](https://doc.rust-lang.org/1.0.0/style/) and these additional conventions:

#### Formatting

Always format code with `rustfmt`:
```bash
cargo fmt --all
```

Configuration in `rustfmt.toml`:
```toml
edition = "2021"
max_width = 100
use_field_init_shorthand = true
use_try_shorthand = true
reorder_imports = true
reorder_modules = true
```

#### Linting

Code must pass Clippy without warnings:
```bash
cargo clippy --all-features --all-targets -- -D warnings
```

#### Naming Conventions

- **Modules**: `snake_case`
- **Types**: `PascalCase`
- **Functions/Methods**: `snake_case`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Lifetimes**: `'lowercase`
- **Type Parameters**: Single capital letter or `PascalCase`

#### Documentation

Every public item must be documented:

```rust
/// Brief one-line description.
///
/// More detailed explanation of what this does, how it works,
/// and when to use it.
///
/// # Arguments
///
/// * `param1` - Description of first parameter
/// * `param2` - Description of second parameter
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// Describes when this function returns an error
///
/// # Examples
///
/// ```rust
/// use agent_sync::example;
///
/// let result = example(1, 2)?;
/// assert_eq!(result, 3);
/// ```
///
/// # Panics
///
/// Describes when this function panics (if applicable)
pub fn example(param1: i32, param2: i32) -> Result<i32> {
    Ok(param1 + param2)
}
```

#### Error Handling

Use the `thiserror` crate for custom errors:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentSyncError {
    #[error("Redis connection failed: {0}")]
    RedisConnection(#[from] redis::RedisError),

    #[error("Task validation failed: {message}")]
    TaskValidation { message: String },

    #[error("Operation timeout after {seconds} seconds")]
    Timeout { seconds: u64 },
}
```

#### Testing Patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_basic_functionality() {
        // Arrange
        let input = prepare_test_data();

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected_value);
    }

    #[tokio::test]
    async fn test_async_operation() {
        // Test async code
    }

    #[test]
    #[should_panic(expected = "error message")]
    fn test_panic_condition() {
        // Test panic scenarios
    }
}
```

## Testing Guidelines

### Test Categories

1. **Unit Tests**: Test individual functions/methods
   ```bash
   cargo test --lib
   ```

2. **Integration Tests**: Test component interactions
   ```bash
   cargo test --test '*'
   ```

3. **Documentation Tests**: Test code in documentation
   ```bash
   cargo test --doc
   ```

4. **Benchmarks**: Performance testing
   ```bash
   cargo bench
   ```

### Writing Tests

#### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::builder()
            .name("test_task")
            .payload(b"data")
            .priority(TaskPriority::High)
            .build();

        assert_eq!(task.name, "test_task");
        assert_eq!(task.priority, TaskPriority::High);
    }
}
```

#### Integration Test Example

Create file in `tests/` directory:

```rust
// tests/redis_integration.rs
use agent_sync::*;
use testcontainers::{clients::Cli, images::redis::Redis};

#[tokio::test]
async fn test_redis_integration() {
    // Start Redis container
    let docker = Cli::default();
    let redis_node = docker.run(Redis::default());
    let port = redis_node.get_host_port_ipv4(6379);

    // Connect and test
    let url = format!("redis://127.0.0.1:{}", port);
    let client = AgentSync::new(Config::default().with_redis_url(&url)).await.unwrap();

    // Test operations
    let task_id = client.submit_task("test", b"data", Default::default()).await.unwrap();
    assert!(!task_id.is_nil());
}
```

### Test Coverage

Maintain minimum 80% test coverage:

```bash
# Generate coverage report
cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

# Generate HTML report
cargo llvm-cov --all-features --workspace --html

# Check coverage threshold
cargo llvm-cov --all-features --workspace --fail-under-lines 80
```

### Property-Based Testing

Use `proptest` for property-based testing:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_task_serialization_roundtrip(
        name in "[a-z]+",
        payload in prop::collection::vec(any::<u8>(), 0..1000),
        priority in 0..4u8,
    ) {
        let task = Task::new(name.clone(), payload.clone())
            .with_priority(priority.into());

        let serialized = serde_json::to_string(&task).unwrap();
        let deserialized: Task = serde_json::from_str(&serialized).unwrap();

        assert_eq!(task.name, deserialized.name);
        assert_eq!(task.payload, deserialized.payload);
    }
}
```

## Documentation Standards

### Code Documentation

- Document all public APIs
- Include examples for complex functions
- Explain "why" not just "what"
- Keep documentation up-to-date with code changes

### README Updates

When adding features, update the README:
- Add to feature list if significant
- Update usage examples
- Add to configuration section if needed
- Update performance characteristics if measured

### API Documentation

Generate and review documentation:
```bash
# Generate docs
cargo doc --all-features --no-deps --open

# Check doc coverage
cargo doc --all-features --no-deps --document-private-items
```

### Changelog

Update `CHANGELOG.md` following [Keep a Changelog](https://keepachangelog.com/):

```markdown
## [Unreleased]

### Added
- New feature description

### Changed
- Modified behavior description

### Fixed
- Bug fix description

### Deprecated
- Deprecated feature notice

### Removed
- Removed feature description

### Security
- Security fix description
```

## Pull Request Process

### Before Submitting

1. **Update your fork**:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Run all checks**:
   ```bash
   # Format code
   cargo fmt --all

   # Run clippy
   cargo clippy --all-features --all-targets -- -D warnings

   # Run tests
   cargo test --all-features

   # Check documentation
   cargo doc --all-features --no-deps

   # Security audit
   cargo audit
   ```

3. **Update documentation** if needed

4. **Add tests** for new functionality

5. **Update CHANGELOG.md**

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Performance improvement
- [ ] Documentation update
- [ ] Refactoring
- [ ] Other (describe)

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Documentation builds
- [ ] Benchmarks run (if applicable)

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] Tests added/updated
- [ ] CHANGELOG.md updated
- [ ] No new warnings

## Related Issues
Fixes #123
Relates to #456
```

### Review Process

1. **Automated checks** must pass
2. **Code review** by maintainers
3. **Address feedback** promptly
4. **Squash commits** if requested
5. **Maintain** until merged

## Release Process

### Version Numbering

Follow [Semantic Versioning](https://semver.org/):
- **MAJOR**: Incompatible API changes
- **MINOR**: Backwards-compatible functionality
- **PATCH**: Backwards-compatible bug fixes

### Release Checklist

1. **Update version** in `Cargo.toml`
2. **Update CHANGELOG.md** with release date
3. **Run full test suite**
4. **Generate documentation**
5. **Create git tag**: `git tag -a v0.1.0 -m "Release v0.1.0"`
6. **Push tag**: `git push upstream v0.1.0`
7. **Publish to crates.io**: `cargo publish`
8. **Create GitHub release** with changelog

### Hotfix Process

For critical bugs in production:

1. Create hotfix branch from tag
2. Fix the issue with minimal changes
3. Test thoroughly
4. Backport to main branch
5. Release patch version

## Getting Help

### Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Async Rust Book](https://rust-lang.github.io/async-book/)
- [Redis Documentation](https://redis.io/documentation)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)

### Communication

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: General questions and ideas
- **Discord**: Real-time chat (if available)
- **Email**: security@anthropic.com (security issues only)

## Recognition

Contributors are recognized in:
- [AUTHORS.md](AUTHORS.md) file
- Release notes
- Project documentation

Thank you for contributing to Agent Sync!