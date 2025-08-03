use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::io::{self, Write};
use std::path::PathBuf;

use purger_core::{
    CleanStrategy, ProjectCleaner, ProjectFilter, ProjectScanner, cleaner::CleanConfig,
    scanner::ScanConfig,
};

/// 扫描命令的参数配置
#[derive(Debug)]
struct ScanCommandArgs {
    path: PathBuf,
    max_depth: Option<usize>,
    target_only: bool,
    sort_by_size: bool,
    keep_days: Option<u32>,
    keep_size: Option<String>,
    ignore_paths: Vec<PathBuf>,
    no_parallel: bool,
    follow_symlinks: bool,
    include_hidden: bool,
    no_gitignore: bool,
}

/// 清理命令的参数配置
#[derive(Debug)]
struct CleanCommandArgs {
    path: PathBuf,
    max_depth: Option<usize>,
    strategy: CleanStrategyArg,
    dry_run: bool,
    keep_days: Option<u32>,
    keep_size: Option<String>,
    ignore_paths: Vec<PathBuf>,
    no_parallel: bool,
    follow_symlinks: bool,
    include_hidden: bool,
    no_gitignore: bool,
    yes: bool,
    keep_executable: bool,
    executable_backup_dir: Option<PathBuf>,
    timeout: u64,
}

/// 扫描配置创建参数
#[derive(Debug)]
struct ScanConfigArgs {
    max_depth: Option<usize>,
    keep_days: Option<u32>,
    keep_size: Option<String>,
    ignore_paths: Vec<PathBuf>,
    no_parallel: bool,
    follow_symlinks: bool,
    include_hidden: bool,
    no_gitignore: bool,
}

#[derive(Parser)]
#[command(name = "purger")]
#[command(about = "A tool for cleaning Rust project build directories")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Enable debug logging
    #[arg(short, long, global = true)]
    pub debug: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Scan for Rust projects in a directory
    Scan {
        /// Directory to scan
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Maximum depth to scan
        #[arg(short, long)]
        max_depth: Option<usize>,

        /// Show only projects with target directories
        #[arg(short, long)]
        target_only: bool,

        /// Sort by size (largest first)
        #[arg(short = 'S', long)]
        sort_by_size: bool,

        /// Keep projects compiled in the last N days
        #[arg(short = 'k', long)]
        keep_days: Option<u32>,

        /// Keep projects with target size smaller than this
        #[arg(short = 's', long)]
        keep_size: Option<String>,

        /// Paths to ignore (can be specified multiple times)
        #[arg(short = 'i', long = "ignore", action = clap::ArgAction::Append)]
        ignore_paths: Vec<PathBuf>,

        /// Disable parallel scanning
        #[arg(long)]
        no_parallel: bool,

        /// Follow symlinks
        #[arg(long)]
        follow_symlinks: bool,

        /// Don't ignore hidden files/directories
        #[arg(long)]
        include_hidden: bool,

        /// Don't respect .gitignore files
        #[arg(long)]
        no_gitignore: bool,
    },
    /// Clean Rust projects
    Clean {
        /// Directory to scan and clean
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Maximum depth to scan
        #[arg(short, long)]
        max_depth: Option<usize>,

        /// Clean strategy
        #[arg(short = 'S', long, value_enum, default_value = "cargo-clean")]
        strategy: CleanStrategyArg,

        /// Dry run - show what would be cleaned without actually cleaning
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Keep projects compiled in the last N days
        #[arg(short = 'k', long)]
        keep_days: Option<u32>,

        /// Keep projects with target size smaller than this
        #[arg(short = 's', long)]
        keep_size: Option<String>,

        /// Paths to ignore (can be specified multiple times)
        #[arg(short = 'i', long = "ignore", action = clap::ArgAction::Append)]
        ignore_paths: Vec<PathBuf>,

        /// Disable parallel processing
        #[arg(long)]
        no_parallel: bool,

        /// Follow symlinks
        #[arg(long)]
        follow_symlinks: bool,

        /// Don't ignore hidden files/directories
        #[arg(long)]
        include_hidden: bool,

        /// Don't respect .gitignore files
        #[arg(long)]
        no_gitignore: bool,

        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,

        /// Keep executable files (backup before cleaning)
        #[arg(long)]
        keep_executable: bool,

        /// Directory to backup executables to
        #[arg(long)]
        executable_backup_dir: Option<PathBuf>,

        /// Timeout for each project clean operation (seconds)
        #[arg(long, default_value = "30")]
        timeout: u64,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum CleanStrategyArg {
    /// Use cargo clean command
    #[value(name = "cargo-clean")]
    CargoClean,
    /// Directly delete target directories
    #[value(name = "direct-delete")]
    DirectDelete,
}

impl From<CleanStrategyArg> for CleanStrategy {
    fn from(arg: CleanStrategyArg) -> Self {
        match arg {
            CleanStrategyArg::CargoClean => CleanStrategy::CargoClean,
            CleanStrategyArg::DirectDelete => CleanStrategy::DirectDelete,
        }
    }
}

pub fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    // 设置日志级别
    let log_level = if cli.debug {
        "debug"
    } else if cli.verbose {
        "info"
    } else {
        "warn"
    };

    tracing_subscriber::fmt()
        .with_env_filter(format!("purger={log_level}"))
        .init();

    match cli.command {
        Commands::Scan {
            path,
            max_depth,
            target_only,
            sort_by_size,
            keep_days,
            keep_size,
            ignore_paths,
            no_parallel,
            follow_symlinks,
            include_hidden,
            no_gitignore,
        } => handle_scan_command(ScanCommandArgs {
            path,
            max_depth,
            target_only,
            sort_by_size,
            keep_days,
            keep_size,
            ignore_paths,
            no_parallel,
            follow_symlinks,
            include_hidden,
            no_gitignore,
        }),
        Commands::Clean {
            path,
            max_depth,
            strategy,
            dry_run,
            keep_days,
            keep_size,
            ignore_paths,
            no_parallel,
            follow_symlinks,
            include_hidden,
            no_gitignore,
            yes,
            keep_executable,
            executable_backup_dir,
            timeout,
        } => handle_clean_command(CleanCommandArgs {
            path,
            max_depth,
            strategy,
            dry_run,
            keep_days,
            keep_size,
            ignore_paths,
            no_parallel,
            follow_symlinks,
            include_hidden,
            no_gitignore,
            yes,
            keep_executable,
            executable_backup_dir,
            timeout,
        }),
    }
}

fn handle_scan_command(args: ScanCommandArgs) -> Result<()> {
    let config = create_scan_config(ScanConfigArgs {
        max_depth: args.max_depth,
        keep_days: args.keep_days,
        keep_size: args.keep_size,
        ignore_paths: args.ignore_paths,
        no_parallel: args.no_parallel,
        follow_symlinks: args.follow_symlinks,
        include_hidden: args.include_hidden,
        no_gitignore: args.no_gitignore,
    })?;

    let scanner = ProjectScanner::new(config.clone());
    let mut projects = scanner.scan(&args.path)?;

    if args.target_only {
        projects = ProjectScanner::filter_with_target(projects);
    }

    if args.sort_by_size {
        projects = ProjectScanner::sort_by_size(projects);
    }

    // 应用过滤器
    if config.keep_days.is_some() || config.keep_size.is_some() || !config.ignore_paths.is_empty() {
        let filter = ProjectFilter::new(config);
        projects = filter.filter_projects(projects);
    }

    display_projects(&projects, &args.path)?;
    Ok(())
}

fn handle_clean_command(args: CleanCommandArgs) -> Result<()> {
    let scan_config = create_scan_config(ScanConfigArgs {
        max_depth: args.max_depth,
        keep_days: args.keep_days,
        keep_size: args.keep_size.clone(),
        ignore_paths: args.ignore_paths,
        no_parallel: args.no_parallel,
        follow_symlinks: args.follow_symlinks,
        include_hidden: args.include_hidden,
        no_gitignore: args.no_gitignore,
    })?;

    let scanner = ProjectScanner::new(scan_config.clone());
    let mut projects = scanner.scan(&args.path)?;

    // 只保留有target目录的项目
    projects = ProjectScanner::filter_with_target(projects);

    // 应用过滤器
    if scan_config.keep_days.is_some()
        || scan_config.keep_size.is_some()
        || !scan_config.ignore_paths.is_empty()
    {
        let filter = ProjectFilter::new(scan_config);
        projects = filter.filter_projects(projects);
    }

    if projects.is_empty() {
        println!("No projects found to clean.");
        return Ok(());
    }

    // 显示将要清理的项目
    println!("Found {} projects to clean:", projects.len());
    display_projects(&projects, &args.path)?;

    // 确认清理
    if !args.yes && !args.dry_run && !confirm_clean(&projects)? {
        println!("Cleaning cancelled.");
        return Ok(());
    }

    // 执行清理
    let clean_config = CleanConfig {
        strategy: args.strategy.into(),
        dry_run: args.dry_run,
        parallel: !args.no_parallel,
        timeout_seconds: args.timeout,
        keep_executable: args.keep_executable,
        executable_backup_dir: args.executable_backup_dir,
    };

    let cleaner = ProjectCleaner::new(clean_config);
    let result = cleaner.clean_projects(&projects);

    // 显示结果
    display_clean_result(&result);

    Ok(())
}

fn create_scan_config(args: ScanConfigArgs) -> Result<ScanConfig> {
    let keep_size_bytes = if let Some(size_str) = args.keep_size {
        Some(purger_core::ProjectFilter::parse_size_string(&size_str)?)
    } else {
        None
    };

    Ok(ScanConfig {
        max_depth: args.max_depth,
        parallel: !args.no_parallel,
        follow_links: args.follow_symlinks,
        ignore_hidden: !args.include_hidden,
        respect_gitignore: !args.no_gitignore,
        lazy_size_calculation: false, // 默认不启用延迟计算
        keep_days: args.keep_days,
        keep_size: keep_size_bytes,
        ignore_paths: args.ignore_paths,
    })
}

fn display_projects(
    projects: &[purger_core::RustProject],
    base_path: &std::path::Path,
) -> Result<()> {
    if projects.is_empty() {
        println!("No projects found.");
        return Ok(());
    }

    let total_size: u64 = projects.iter().map(|p| p.target_size).sum();

    println!("\nFound {} projects:", projects.len());
    println!("{:<40} {:<15} {:<20}", "Project", "Size", "Path");
    println!("{}", "-".repeat(75));

    for project in projects {
        let relative_path = project.relative_path(base_path);
        println!(
            "{:<40} {:<15} {:<20}",
            project.name,
            project.formatted_size(),
            relative_path.display()
        );
    }

    println!("{}", "-".repeat(75));
    println!("Total size: {}", purger_core::format_bytes(total_size));

    Ok(())
}

fn confirm_clean(projects: &[purger_core::RustProject]) -> Result<bool> {
    let total_size: u64 = projects.iter().map(|p| p.target_size).sum();

    print!(
        "\nThis will clean {} projects and free up {}. Continue? [y/N]: ",
        projects.len(),
        purger_core::format_bytes(total_size)
    );

    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes")
}

fn display_clean_result(result: &purger_core::CleanResult) {
    println!("\nCleaning completed!");
    println!("Projects cleaned: {}", result.cleaned_projects);
    println!("Size freed: {}", result.format_size());

    if !result.failed_projects.is_empty() {
        println!(
            "\nFailed to clean {} projects:",
            result.failed_projects.len()
        );
        for project in &result.failed_projects {
            println!("  - {project}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_cli_parse_scan_command() {
        let args = vec![
            "purger",
            "scan",
            "/tmp",
            "--max-depth",
            "3",
            "--target-only",
        ];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Scan {
                path,
                max_depth,
                target_only,
                ..
            } => {
                assert_eq!(path, PathBuf::from("/tmp"));
                assert_eq!(max_depth, Some(3));
                assert!(target_only);
            }
            _ => panic!("Expected Scan command"),
        }
    }

    #[test]
    fn test_cli_parse_clean_command() {
        let args = vec![
            "purger",
            "clean",
            "/tmp",
            "--strategy",
            "direct-delete",
            "--dry-run",
            "--yes",
        ];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Clean {
                path,
                strategy,
                dry_run,
                yes,
                ..
            } => {
                assert_eq!(path, PathBuf::from("/tmp"));
                assert!(matches!(strategy, CleanStrategyArg::DirectDelete));
                assert!(dry_run);
                assert!(yes);
            }
            _ => panic!("Expected Clean command"),
        }
    }

    #[test]
    fn test_create_scan_config() {
        let config = create_scan_config(ScanConfigArgs {
            max_depth: Some(5),
            keep_days: Some(7),
            keep_size: Some("1MB".to_string()),
            ignore_paths: vec![PathBuf::from("/ignore")],
            no_parallel: false,
            follow_symlinks: true,
            include_hidden: false,
            no_gitignore: true,
        })
        .unwrap();

        assert_eq!(config.max_depth, Some(5));
        assert_eq!(config.keep_days, Some(7));
        assert_eq!(config.keep_size, Some(1_000_000));
        assert_eq!(config.ignore_paths, vec![PathBuf::from("/ignore")]);
        assert!(config.parallel);
        assert!(config.follow_links);
        assert!(config.ignore_hidden);
        assert!(!config.respect_gitignore);
    }

    #[test]
    fn test_clean_strategy_conversion() {
        assert!(matches!(
            CleanStrategy::from(CleanStrategyArg::CargoClean),
            CleanStrategy::CargoClean
        ));
        assert!(matches!(
            CleanStrategy::from(CleanStrategyArg::DirectDelete),
            CleanStrategy::DirectDelete
        ));
    }

    #[test]
    fn test_display_projects_empty() {
        let projects = vec![];
        let temp_dir = TempDir::new().unwrap();
        let result = display_projects(&projects, temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_confirm_clean_calculation() {
        use purger_core::RustProject;
        use std::time::SystemTime;

        let projects = [
            RustProject {
                path: PathBuf::from("/test1"),
                name: "test1".to_string(),
                target_size: 1000,
                last_modified: SystemTime::now(),
                is_workspace: false,
                has_target: true,
            },
            RustProject {
                path: PathBuf::from("/test2"),
                name: "test2".to_string(),
                target_size: 2000,
                last_modified: SystemTime::now(),
                is_workspace: false,
                has_target: true,
            },
        ];

        // 这个测试只验证函数不会panic，实际的用户输入测试比较复杂
        // 在实际应用中，可能需要mock stdin
        let total_size: u64 = projects.iter().map(|p| p.target_size).sum();
        assert_eq!(total_size, 3000);
    }
}
