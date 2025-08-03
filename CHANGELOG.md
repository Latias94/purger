# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- Optimized directory scanning performance with parallel traversal, lazy size calculation, and TOML parsing improvements

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

[Unreleased]: https://github.com/Latias94/purger/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Latias94/purger/releases/tag/v0.1.0
