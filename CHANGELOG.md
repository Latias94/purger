# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.1] - 2026-01-18

### Fixed
- Non-Windows build failure in `purger-core` due to missing `warn!` macro import

## [0.4.0] - 2026-01-18

### Added
- Single `purger` executable that includes both GUI and CLI (GUI launches by default; CLI subcommands unchanged)
- Windows turbo direct-delete backend via `cmd.exe rmdir /S /Q` (configurable in CLI/GUI; falls back automatically on failure)
- Richer GUI sorting & filtering (workspace-only filter, more sort orders, table-header click sorting)

### Changed
- GUI scanning is now cancellable and reports real progress
- GUI loads project list quickly and backfills target sizes in the background
- Clean timeout default is now disabled (`0` seconds)
- Workspace dependencies upgraded (notably `egui/eframe 0.33`, `rfd 0.17`, `tokio 1.49`)

### Fixed
- Scan no longer aborts when encountering a broken `Cargo.toml`
- Filters side panel no longer grows continuously due to unconstrained widget widths

### Performance
- Virtualized project table rendering for large project sets
- Faster cancellable direct delete via chunked parallel deletion

## [0.3.0] - 2025-08-04

### Changed
- GUI layout redesign and improved cleaning UX
- Improved scan performance and target size handling

## [0.1.0] - 2025-08-03

### Added
- CLI interface for scanning and cleaning Rust project build directories
- GUI interface with internationalization support
- Core library with project scanning and cleaning functionality
- Support for workspace projects
- Configurable scanning options (depth, gitignore, hidden files)
- Multiple cleaning strategies (cargo clean, direct deletion)
- Parallel processing support
- Size-based and time-based filtering
- Executable backup functionality

### Features
- **CLI**: Command-line interface with comprehensive options
- **GUI**: Cross-platform graphical interface with i18n support
- **Core**: Robust scanning and cleaning engine
- **Configuration**: Flexible configuration system
- **Performance**: Parallel processing and optimized algorithms

[Unreleased]: https://github.com/Latias94/purger/compare/v0.4.1...HEAD
[0.4.1]: https://github.com/Latias94/purger/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/Latias94/purger/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/Latias94/purger/compare/v0.1.0...v0.3.0
[0.1.0]: https://github.com/Latias94/purger/releases/tag/v0.1.0
