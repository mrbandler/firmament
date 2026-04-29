# Contributing to Firmament

Thanks for your interest in contributing to Firmament! This document covers everything you need to get started.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Making Changes](#making-changes)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Submitting Changes](#submitting-changes)
- [Release Process](#release-process)

## Code of Conduct

This project follows a [Code of Conduct](./CODE_OF_CONDUCT.md). By participating you agree to uphold it. Be respectful and constructive in all interactions.

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [just](https://github.com/casey/just) (command runner)
- Git
- A code editor (we recommend VS Code with rust-analyzer)

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork locally:

   ```bash
   git clone https://github.com/YOUR_USERNAME/firmament.git
   cd firmament
   ```

3. Add the upstream repository:

   ```bash
   git remote add upstream https://github.com/mrbandler/firmament.git
   ```

## Development Setup

The project uses a `justfile` to automate common development tasks. After cloning, run:

```bash
just setup
```

This will:

- Install the Rust toolchain from `rust-toolchain.toml` (stable + clippy, rustfmt, rust-analyzer, wasm32 target)
- Install `cargo-deny` (dependency auditing)
- Install `mdbook` (documentation site)
- Install [prek](https://github.com/j178/prek) (pre-commit hook runner)
- Fetch all crate dependencies
- Register the git pre-commit hooks

### Verify Setup

```bash
just check
```

This runs formatting, clippy, tests, dependency audit, and doc build in sequence.

## Project Structure

```
firmament/
├── crates/
│   ├── firmament-core/    # Host-side runtime (executor, WASM engine, virtual MCU)
│   └── firmament-fm/      # Guest-side firmware library (no_std, WASM imports)
├── examples/
│   └── blink/             # Minimal firmware example (LED blink via MMIO)
├── docs/                  # mdBook documentation site
├── Cargo.toml             # Workspace configuration
├── justfile               # Development task runner
├── rustfmt.toml           # Formatting rules
├── clippy.toml            # Lint configuration
├── cliff.toml             # Changelog generation (git-cliff)
├── deny.toml              # Dependency audit rules
├── rust-toolchain.toml    # Rust version pinning
└── .pre-commit-config.yaml # Pre-commit hook configuration
```

## Making Changes

### Branch Naming

Use descriptive branch names:

- `feat/description` - New features
- `fix/description` - Bug fixes
- `docs/description` - Documentation changes
- `refactor/description` - Code refactoring
- `test/description` - Test additions or changes

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description

[optional body]

[optional footer]
```

Types:

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting (no code change)
- `refactor`: Code restructuring
- `test`: Adding tests
- `chore`: Maintenance tasks

Examples:

```
feat(core): add watchdog peripheral support
fix(executor): handle firmware trap on budget exhaustion
docs(readme): update quick start instructions
refactor(mcu): extract channel setup into dedicated module
test(core): add lifecycle state transition tests
```

## Coding Standards

### Formatting

All code must be formatted with `rustfmt`:

```bash
just fmt
```

Configuration is in `rustfmt.toml`.

### Linting

All code must pass `clippy` without warnings:

```bash
just clippy
```

Configuration is in `clippy.toml` and `Cargo.toml` workspace lints.

To run all pre-commit hooks (formatting, clippy, file checks, etc.) manually:

```bash
just lint
```

### Documentation

- All public items must have documentation comments
- Use `///` for item documentation
- Use `//!` for module-level documentation
- Include examples in doc comments where appropriate
- Run `just doc` to verify docs build

### Error Handling

- Use `miette` for user-facing errors with helpful diagnostics
- Use `thiserror` for library error types
- Provide context with `.context()` or `.with_context()`
- Never use `.unwrap()` in library code (okay in tests)

### Code Style

- Prefer `impl Trait` over `dyn Trait` where possible
- Use `#[must_use]` for functions with important return values
- Prefer iterators over explicit loops
- Keep functions focused and under 100 lines
- Use meaningful variable names

## Testing

### Running Tests

```bash
# All tests
just test

# With extra arguments
just test -- --nocapture

# Specific crate
cargo test --package firmament-core

# Specific test
cargo test --package firmament-core test_name
```

### Writing Tests

- Place unit tests in a `#[cfg(test)] mod tests` block
- Place integration tests in `tests/` directory
- Use descriptive test names: `test_<function>_<scenario>_<expected>`
- Test both success and error cases

Example:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcu_starts_in_off_state() {
        let mcu = TestMcu::new();
        let handle = mcu.spawn().await.unwrap();
        assert_eq!(handle.state(), McuState::Off);
    }
}
```

## Submitting Changes

### Before Submitting

1. Make sure all checks pass:

   ```bash
   just check
   ```

2. Update documentation if needed
3. Add tests for new functionality
4. Update CHANGELOG.md (if applicable)

### Pull Request Process

1. Update your branch with the latest upstream changes:

   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. Push your branch and create a pull request

3. Fill out the PR template with:
   - Description of changes
   - Related issues
   - Testing performed

4. Wait for CI to pass

5. Address review feedback

6. Once approved, a maintainer will merge your PR

### Pull Request Checklist

- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] Code is formatted with `rustfmt`
- [ ] Code passes `clippy` lints
- [ ] Documentation is updated
- [ ] CHANGELOG.md is updated (for notable changes)
- [ ] Commit messages follow conventions

## Release Process

Releases are automated via GitHub Actions when a version tag is pushed.

### Version Bumping

1. Update version in `Cargo.toml` (workspace manages this)
2. Update `CHANGELOG.md` with release notes
3. Commit: `git commit -am "chore: bump version to x.y.z"`
4. Tag: `git tag vx.y.z`
5. Push: `git push && git push --tags`

### Versioning

We follow [Semantic Versioning](https://semver.org/):

- MAJOR: Breaking changes
- MINOR: New features (backward compatible)
- PATCH: Bug fixes (backward compatible)

## Getting Help

- Open an issue for bugs or feature requests
- Start a discussion for questions or ideas
- Check existing issues and discussions first

Thanks for contributing to Firmament!
