use purger_core::{CleanProgress, ProjectCleaner, cleaner::CleanConfig};
use std::fs;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建临时测试项目
    let temp_dir = TempDir::new()?;
    let project_dir = temp_dir.path().join("test_project");
    fs::create_dir_all(&project_dir)?;

    // 创建 Cargo.toml
    let cargo_toml = r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "test_project"
path = "src/main.rs"
"#;
    fs::write(project_dir.join("Cargo.toml"), cargo_toml)?;

    // 创建 src 目录和 main.rs
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src").join("main.rs"),
        "fn main() { println!(\"Hello, world!\"); }",
    )?;

    // 创建 target 目录和一些文件
    let target_dir = project_dir.join("target");
    fs::create_dir_all(target_dir.join("debug"))?;
    fs::write(target_dir.join("debug").join("test.exe"), "fake executable")?;
    fs::write(target_dir.join("CACHEDIR.TAG"), "cache tag")?;

    // 创建项目实例
    let project = purger_core::RustProject::from_path(&project_dir)?;

    println!("测试项目: {}", project.name);
    println!("Target 大小: {} bytes", project.target_size);

    // 收集进度信息
    let progress_log = Arc::new(Mutex::new(Vec::<CleanProgress>::new()));
    let progress_log_clone = progress_log.clone();

    // 创建清理器并执行清理
    let config = CleanConfig::default();
    let cleaner = ProjectCleaner::new(config);

    println!("\n开始清理...");
    let result = cleaner.clean_project_with_progress(&project, |progress| {
        println!(
            "进度: {} - {:?} - {}/{}",
            progress.project_name,
            progress.phase,
            progress.files_processed,
            progress.total_files.unwrap_or(0)
        );

        if let Some(file) = &progress.current_file {
            println!("  当前文件: {file}");
        }

        progress_log_clone.lock().unwrap().push(progress);
    });

    match result {
        Ok(size_freed) => {
            println!("\n清理成功! 释放空间: {size_freed} bytes");
        }
        Err(e) => {
            println!("\n清理失败: {e}");
        }
    }

    // 显示进度日志
    let log = progress_log.lock().unwrap();
    println!("\n进度日志 ({} 条记录):", log.len());
    for (i, progress) in log.iter().enumerate() {
        println!(
            "  {}: {:?} - {}/{}",
            i + 1,
            progress.phase,
            progress.files_processed,
            progress.total_files.unwrap_or(0)
        );
    }

    Ok(())
}
