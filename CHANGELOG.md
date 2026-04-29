# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Virtual MCU runtime with cycle-accurate budget execution
  - WASM-based firmware loading and linking via Wasmtime
  - Cycle budget system with configurable compute-per-cycle ratios
  - Memory-mapped I/O (MMIO) with volatile read/write semantics
  - Interrupt support with firmware-exported ISR handlers
  - Cold and warm reset via runtime handle
- Asynchronous command/event channel architecture
  - `Handle` for sending commands (tick, reset, halt, resume) to a running MCU
  - Event stream for observing state changes and firmware traps
- `firmament-fm` guest-side firmware library
  - `no_std` WASM import declarations for MMIO, sleep, and debug logging
  - Volatile access primitives for register-level I/O
- Blink example demonstrating LED control via MMIO registers
- Development tooling: justfile, pre-commit hooks, cargo-deny, clippy, rustfmt
- mdBook documentation scaffold

<!--
## [0.1.0] - YYYY-MM-DD

### Added

- Initial release
-->

[Unreleased]: https://github.com/mrbandler/firmament/compare/HEAD...HEAD
<!-- [0.1.0]: https://github.com/mrbandler/firmament/releases/tag/v0.1.0 -->
