//! # Purger
//!
//! A tool for cleaning Rust project build directories.
//!
//! This crate provides both command-line and GUI interfaces for scanning
//! and cleaning Rust project target directories to free up disk space.
//!
//! ## Features
//!
//! - Scan directories for Rust projects
//! - Clean target directories using `cargo clean` or direct deletion
//! - Progress tracking for cleaning operations
//! - Both CLI and GUI interfaces
//!
//! ## Usage
//!
//! ### Command Line
//!
//! ```bash
//! # Scan current directory
//! purger scan
//!
//! # Clean all projects in current directory
//! purger clean --all
//!
//! # Dry run to see what would be cleaned
//! purger clean --dry-run
//! ```
//!
//! ### As a Library
//!
//! ```rust
//! use purger_core::{ProjectScanner, ProjectCleaner, scanner::ScanConfig, cleaner::CleanConfig};
//! use std::path::Path;
//!
//! // Scan for projects
//! let scanner = ProjectScanner::new(ScanConfig::default());
//! let projects = scanner.scan(Path::new("."))?;
//!
//! // Clean projects (using dry_run to avoid actual deletion)
//! let mut clean_config = CleanConfig::default();
//! clean_config.dry_run = true; // Use dry run to avoid permission issues
//! let cleaner = ProjectCleaner::new(clean_config);
//! for project in &projects {
//!     let _ = cleaner.clean_project(project); // Ignore result in doc test
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

// Re-export core functionality
pub use purger_core::*;

// Re-export commonly used types
pub use purger_core::{
    cleaner::CleanConfig, scanner::ScanConfig, CleanPhase, CleanProgress, CleanResult,
    CleanStrategy, ProjectCleaner, ProjectScanner, RustProject,
};
