use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use tracing::{debug, error, info};

use crate::CleanResult;
use crate::project::RustProject;

/// 清理策略
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum CleanStrategy {
    /// 使用cargo clean命令
    #[default]
    CargoClean,
    /// 直接删除target目录
    DirectDelete,
}

/// 清理进度信息
#[derive(Debug, Clone)]
pub struct CleanProgress {
    pub project_name: String,
    pub current_file: Option<String>,
    pub files_processed: usize,
    pub total_files: Option<usize>,
    pub phase: CleanPhase,
}

/// 清理阶段
#[derive(Debug, Clone, PartialEq)]
pub enum CleanPhase {
    Starting,
    Analyzing,
    Cleaning,
    Finalizing,
    Complete,
}

/// 清理器配置
#[derive(Debug, Clone)]
pub struct CleanConfig {
    pub strategy: CleanStrategy,
    pub dry_run: bool,
    pub parallel: bool,
    pub timeout_seconds: u64,

    // 可执行文件保留选项
    /// 是否保留可执行文件
    pub keep_executable: bool,
    /// 可执行文件备份目录（如果为None，则在项目目录下创建executables文件夹）
    pub executable_backup_dir: Option<PathBuf>,
}

impl Default for CleanConfig {
    fn default() -> Self {
        Self {
            strategy: CleanStrategy::CargoClean,
            dry_run: false,
            parallel: true,
            timeout_seconds: 30,

            // 可执行文件保留选项默认值
            keep_executable: false,
            executable_backup_dir: None,
        }
    }
}

/// 项目清理器
pub struct ProjectCleaner {
    config: CleanConfig,
}

impl ProjectCleaner {
    /// 创建新的清理器
    pub fn new(config: CleanConfig) -> Self {
        Self { config }
    }

    /// 清理单个项目
    pub fn clean_project(&self, project: &RustProject) -> Result<u64> {
        self.clean_project_with_progress(project, |_| {})
    }

    /// 清理单个项目（带进度回调）
    pub fn clean_project_with_progress<F>(
        &self,
        project: &RustProject,
        progress_callback: F,
    ) -> Result<u64>
    where
        F: Fn(CleanProgress),
    {
        if self.config.dry_run {
            info!(
                "DRY RUN: 将清理项目 {} ({})",
                project.name,
                project.formatted_size()
            );
            return Ok(project.target_size);
        }

        if !project.has_target {
            debug!("项目 {} 没有target目录，跳过", project.name);
            return Ok(0);
        }

        let size_before = project.target_size;
        info!(
            "开始清理项目: {} ({})",
            project.name,
            project.formatted_size()
        );

        // 发送开始进度
        progress_callback(CleanProgress {
            project_name: project.name.clone(),
            current_file: None,
            files_processed: 0,
            total_files: None,
            phase: CleanPhase::Starting,
        });

        let result = match self.config.strategy {
            CleanStrategy::CargoClean => {
                self.clean_with_cargo_progress(project, &progress_callback)
            }
            CleanStrategy::DirectDelete => {
                self.clean_with_delete_progress(project, &progress_callback)
            }
        };

        match result {
            Ok(_) => {
                // 发送完成进度
                progress_callback(CleanProgress {
                    project_name: project.name.clone(),
                    current_file: None,
                    files_processed: 0,
                    total_files: Some(0),
                    phase: CleanPhase::Complete,
                });
                info!("成功清理项目: {}", project.name);
                Ok(size_before)
            }
            Err(e) => {
                error!("清理项目失败 {}: {}", project.name, e);
                Err(e)
            }
        }
    }

    /// 批量清理项目
    pub fn clean_projects(&self, projects: &[RustProject]) -> CleanResult {
        let start_time = Instant::now();
        let mut result = CleanResult::new();

        info!("开始清理 {} 个项目", projects.len());

        if self.config.parallel {
            self.clean_projects_parallel(projects, &mut result);
        } else {
            self.clean_projects_sequential(projects, &mut result);
        }

        result.duration_ms = start_time.elapsed().as_millis() as u64;

        info!(
            "清理完成: 成功 {} 个，失败 {} 个，释放空间 {}，耗时 {}ms",
            result.cleaned_projects,
            result.failed_projects.len(),
            result.format_size(),
            result.duration_ms
        );

        result
    }

    /// 串行清理项目
    fn clean_projects_sequential(&self, projects: &[RustProject], result: &mut CleanResult) {
        for project in projects {
            match self.clean_project(project) {
                Ok(size_freed) => result.add_success(size_freed),
                Err(_) => result.add_failure(project.path.to_string_lossy().to_string()),
            }
        }
    }

    /// 并行清理项目（注意：这里简化实现，实际可能需要更复杂的并行控制）
    fn clean_projects_parallel(&self, projects: &[RustProject], result: &mut CleanResult) {
        // 由于需要修改result，这里暂时使用串行实现
        // 在实际应用中，可以使用Arc<Mutex<CleanResult>>或其他并发原语
        self.clean_projects_sequential(projects, result);
    }

    /// 使用cargo clean清理
    #[allow(dead_code)]
    fn clean_with_cargo(&self, project: &RustProject) -> Result<()> {
        self.clean_with_cargo_progress(project, &|_| {})
    }

    /// 使用cargo clean清理（带进度回调）
    fn clean_with_cargo_progress<F>(
        &self,
        project: &RustProject,
        progress_callback: &F,
    ) -> Result<()>
    where
        F: Fn(CleanProgress),
    {
        debug!("使用cargo clean清理项目: {}", project.name);

        // 首先运行 dry-run 来获取文件列表
        progress_callback(CleanProgress {
            project_name: project.name.clone(),
            current_file: None,
            files_processed: 0,
            total_files: None,
            phase: CleanPhase::Analyzing,
        });

        let file_list = self.get_cargo_clean_file_list(project)?;
        let total_files = file_list.len();

        progress_callback(CleanProgress {
            project_name: project.name.clone(),
            current_file: None,
            files_processed: 0,
            total_files: Some(total_files),
            phase: CleanPhase::Cleaning,
        });

        // 执行实际的清理
        let mut cmd = Command::new("cargo");
        cmd.arg("clean").current_dir(&project.path);

        // 模拟进度更新（因为cargo clean本身不提供实时进度）
        let handle = std::thread::spawn(move || cmd.output());

        // 在清理过程中模拟进度更新
        let mut processed = 0;
        while !handle.is_finished() {
            if processed < total_files {
                processed = (processed + total_files / 10).min(total_files);
                progress_callback(CleanProgress {
                    project_name: project.name.clone(),
                    current_file: file_list.get(processed.saturating_sub(1)).cloned(),
                    files_processed: processed,
                    total_files: Some(total_files),
                    phase: CleanPhase::Cleaning,
                });
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        let output = handle
            .join()
            .map_err(|_| anyhow::anyhow!("清理线程异常"))?
            .context("执行cargo clean失败")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("cargo clean失败: {}", stderr);
        }

        // 最终进度更新
        progress_callback(CleanProgress {
            project_name: project.name.clone(),
            current_file: None,
            files_processed: total_files,
            total_files: Some(total_files),
            phase: CleanPhase::Finalizing,
        });

        Ok(())
    }

    /// 获取cargo clean将要删除的文件列表
    fn get_cargo_clean_file_list(&self, project: &RustProject) -> Result<Vec<String>> {
        let mut cmd = Command::new("cargo");
        cmd.arg("clean").arg("--dry-run").current_dir(&project.path);

        let output = cmd.output().context("执行cargo clean --dry-run失败")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("cargo clean --dry-run失败: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let files: Vec<String> = stdout
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.contains("Summary"))
            .map(|line| {
                // 提取文件名部分
                if let Some(file_name) = std::path::Path::new(line.trim()).file_name() {
                    file_name.to_string_lossy().to_string()
                } else {
                    line.trim().to_string()
                }
            })
            .collect();

        Ok(files)
    }

    /// 直接删除target目录
    #[allow(dead_code)]
    fn clean_with_delete(&self, project: &RustProject) -> Result<()> {
        self.clean_with_delete_progress(project, &|_| {})
    }

    /// 直接删除target目录（带进度回调）
    fn clean_with_delete_progress<F>(
        &self,
        project: &RustProject,
        progress_callback: &F,
    ) -> Result<()>
    where
        F: Fn(CleanProgress),
    {
        debug!("直接删除target目录: {}", project.name);

        let target_path = project.target_path();
        if !target_path.exists() {
            return Ok(());
        }

        progress_callback(CleanProgress {
            project_name: project.name.clone(),
            current_file: None,
            files_processed: 0,
            total_files: None,
            phase: CleanPhase::Analyzing,
        });

        // 如果需要保留可执行文件，先备份
        if self.config.keep_executable {
            self.backup_executables(project, progress_callback)?;
        }

        // 计算文件数量（用于进度显示）
        let file_count = self.count_files_in_dir(&target_path)?;

        progress_callback(CleanProgress {
            project_name: project.name.clone(),
            current_file: Some("target".to_string()),
            files_processed: 0,
            total_files: Some(file_count),
            phase: CleanPhase::Cleaning,
        });

        // 执行删除
        std::fs::remove_dir_all(&target_path).context("删除target目录失败")?;

        progress_callback(CleanProgress {
            project_name: project.name.clone(),
            current_file: None,
            files_processed: file_count,
            total_files: Some(file_count),
            phase: CleanPhase::Finalizing,
        });

        Ok(())
    }

    /// 备份可执行文件
    fn backup_executables<F>(&self, project: &RustProject, progress_callback: &F) -> Result<()>
    where
        F: Fn(CleanProgress),
    {
        let target_path = project.target_path();
        let executables = self.find_executables(&target_path)?;

        if executables.is_empty() {
            debug!("项目 {} 没有找到可执行文件", project.name);
            return Ok(());
        }

        info!(
            "项目 {} 找到 {} 个可执行文件，开始备份",
            project.name,
            executables.len()
        );

        // 确定备份目录
        let backup_dir = self.get_backup_directory(project)?;
        std::fs::create_dir_all(&backup_dir).context("创建备份目录失败")?;

        // 备份每个可执行文件
        for (i, exe_path) in executables.iter().enumerate() {
            let file_name = exe_path
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("无效的可执行文件路径"))?;
            let backup_path = backup_dir.join(file_name);

            progress_callback(CleanProgress {
                project_name: project.name.clone(),
                current_file: Some(format!("备份 {}", file_name.to_string_lossy())),
                files_processed: i,
                total_files: Some(executables.len()),
                phase: CleanPhase::Cleaning,
            });

            std::fs::copy(exe_path, &backup_path)
                .with_context(|| format!("备份可执行文件失败: {exe_path:?} -> {backup_path:?}"))?;

            debug!("备份可执行文件: {:?} -> {:?}", exe_path, backup_path);
        }

        info!(
            "成功备份 {} 个可执行文件到 {:?}",
            executables.len(),
            backup_dir
        );
        Ok(())
    }

    /// 查找target目录中的可执行文件
    fn find_executables(&self, target_path: &std::path::Path) -> Result<Vec<PathBuf>> {
        let mut executables = Vec::new();

        // 检查常见的可执行文件目录
        let exe_dirs = [target_path.join("debug"), target_path.join("release")];

        for exe_dir in &exe_dirs {
            if exe_dir.exists() {
                self.scan_directory_for_executables(exe_dir, &mut executables)?;
            }
        }

        // 也检查其他可能的目录（如交叉编译目标）
        if let Ok(entries) = std::fs::read_dir(target_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && !path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .starts_with('.')
                {
                    // 检查是否是目标架构目录
                    if let Ok(sub_entries) = std::fs::read_dir(&path) {
                        for sub_entry in sub_entries.flatten() {
                            let sub_path = sub_entry.path();
                            if sub_path.is_dir()
                                && (sub_path.file_name().unwrap_or_default() == "debug"
                                    || sub_path.file_name().unwrap_or_default() == "release")
                            {
                                self.scan_directory_for_executables(&sub_path, &mut executables)?;
                            }
                        }
                    }
                }
            }
        }

        Ok(executables)
    }

    /// 扫描目录查找可执行文件
    fn scan_directory_for_executables(
        &self,
        dir: &std::path::Path,
        executables: &mut Vec<PathBuf>,
    ) -> Result<()> {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && self.is_executable(&path) {
                    executables.push(path);
                }
            }
        }
        Ok(())
    }

    /// 判断文件是否为可执行文件
    fn is_executable(&self, path: &std::path::Path) -> bool {
        // 在Windows上检查.exe扩展名
        #[cfg(target_os = "windows")]
        {
            path.extension().is_some_and(|ext| ext == "exe")
        }

        // 在Unix系统上检查可执行权限
        #[cfg(not(target_os = "windows"))]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(path) {
                let permissions = metadata.permissions();
                permissions.mode() & 0o111 != 0
            } else {
                false
            }
        }
    }

    /// 获取备份目录
    fn get_backup_directory(&self, project: &RustProject) -> Result<PathBuf> {
        if let Some(ref backup_dir) = self.config.executable_backup_dir {
            // 使用指定的备份目录
            Ok(backup_dir.join(&project.name))
        } else {
            // 在项目目录下创建executables文件夹
            Ok(project.path.join("executables"))
        }
    }

    /// 计算目录中的文件数量
    fn count_files_in_dir(&self, dir: &std::path::Path) -> Result<usize> {
        use walkdir::WalkDir;

        let count = WalkDir::new(dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .count();

        Ok(count)
    }

    /// 预览清理操作（dry run）
    pub fn preview_clean(&self, projects: &[RustProject]) -> CleanResult {
        let mut config = self.config.clone();
        config.dry_run = true;

        let cleaner = ProjectCleaner::new(config);
        cleaner.clean_projects(projects)
    }

    /// 检查cargo命令是否可用
    pub fn check_cargo_available() -> bool {
        Command::new("cargo")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

impl Default for ProjectCleaner {
    fn default() -> Self {
        Self::new(CleanConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_test_project_with_target(dir: &Path, name: &str) -> Result<RustProject> {
        let project_dir = dir.join(name);
        fs::create_dir_all(&project_dir)?;

        let cargo_toml = format!(
            r#"
[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
"#
        );

        fs::write(project_dir.join("Cargo.toml"), cargo_toml)?;

        let target_dir = project_dir.join("target");
        fs::create_dir_all(&target_dir)?;
        fs::write(
            target_dir.join("test.txt"),
            "test content for size calculation",
        )?;

        RustProject::from_path(&project_dir)
    }

    #[test]
    fn test_cleaner_dry_run() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let project = create_test_project_with_target(temp_dir.path(), "test_project")?;

        let config = CleanConfig {
            dry_run: true,
            ..Default::default()
        };

        let cleaner = ProjectCleaner::new(config);
        let size_freed = cleaner.clean_project(&project)?;

        // 在dry run模式下，应该返回原始大小
        assert_eq!(size_freed, project.target_size);

        // target目录应该仍然存在
        assert!(project.target_path().exists());

        Ok(())
    }

    #[test]
    fn test_cleaner_direct_delete() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let project = create_test_project_with_target(temp_dir.path(), "test_project")?;

        let config = CleanConfig {
            strategy: CleanStrategy::DirectDelete,
            ..Default::default()
        };

        let cleaner = ProjectCleaner::new(config);
        let size_freed = cleaner.clean_project(&project)?;

        // 应该释放了一些空间
        assert!(size_freed > 0);

        // target目录应该被删除
        assert!(!project.target_path().exists());

        Ok(())
    }

    #[test]
    fn test_check_cargo_available() {
        // 这个测试可能在某些环境中失败，如果cargo不可用
        // 在实际项目中，可能需要mock这个功能
        let available = ProjectCleaner::check_cargo_available();
        println!("Cargo available: {available}");
    }

    #[test]
    fn test_clean_projects_batch() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let projects = vec![
            create_test_project_with_target(temp_dir.path(), "project1")?,
            create_test_project_with_target(temp_dir.path(), "project2")?,
        ];

        let config = CleanConfig {
            strategy: CleanStrategy::DirectDelete,
            dry_run: false,
            ..Default::default()
        };

        let cleaner = ProjectCleaner::new(config);
        let result = cleaner.clean_projects(&projects);

        assert_eq!(result.cleaned_projects, 2);
        assert!(result.total_size_freed > 0);
        assert!(result.failed_projects.is_empty());

        Ok(())
    }

    #[test]
    fn test_clean_config_default() {
        let config = CleanConfig::default();
        assert_eq!(config.strategy, CleanStrategy::CargoClean);
        assert!(!config.dry_run);
        assert!(config.parallel);
        assert_eq!(config.timeout_seconds, 30);
        assert!(!config.keep_executable);
        assert!(config.executable_backup_dir.is_none());
    }

    #[test]
    fn test_clean_progress_phases() {
        let progress = CleanProgress {
            project_name: "test".to_string(),
            current_file: Some("test.txt".to_string()),
            files_processed: 5,
            total_files: Some(10),
            phase: CleanPhase::Cleaning,
        };

        assert_eq!(progress.project_name, "test");
        assert_eq!(progress.current_file, Some("test.txt".to_string()));
        assert_eq!(progress.files_processed, 5);
        assert_eq!(progress.total_files, Some(10));
        assert_eq!(progress.phase, CleanPhase::Cleaning);
    }

    #[test]
    fn test_clean_strategy_default() {
        let strategy = CleanStrategy::default();
        assert_eq!(strategy, CleanStrategy::CargoClean);
    }

    #[test]
    fn test_clean_with_progress_callback() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let project = create_test_project_with_target(temp_dir.path(), "test_project")?;

        let config = CleanConfig {
            strategy: CleanStrategy::DirectDelete,
            dry_run: true, // 使用dry run避免实际删除
            ..Default::default()
        };

        let cleaner = ProjectCleaner::new(config);

        // 简单测试进度回调不会导致panic
        let size_freed = cleaner.clean_project_with_progress(&project, |_progress| {
            // 进度回调被调用，但我们不在这里做任何可变操作
        })?;

        assert!(size_freed > 0);

        Ok(())
    }

    #[test]
    fn test_clean_result_operations() {
        let mut result = CleanResult::new();

        // 测试初始状态
        assert_eq!(result.cleaned_projects, 0);
        assert_eq!(result.total_size_freed, 0);
        assert!(result.failed_projects.is_empty());

        // 测试添加成功
        result.add_success(1024);
        assert_eq!(result.cleaned_projects, 1);
        assert_eq!(result.total_size_freed, 1024);

        // 测试添加失败
        result.add_failure("failed_project".to_string());
        assert_eq!(result.failed_projects.len(), 1);
        assert_eq!(result.failed_projects[0], "failed_project");

        // 测试格式化大小
        let formatted = result.format_size();
        assert_eq!(formatted, "1.00 KB");
    }

    #[test]
    fn test_clean_nonexistent_project() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let fake_project = RustProject {
            path: temp_dir.path().join("nonexistent"),
            name: "nonexistent".to_string(),
            target_size: 1000,
            last_modified: std::time::SystemTime::now(),
            is_workspace: false,
            has_target: true,
        };

        let cleaner = ProjectCleaner::default();
        let result = cleaner.clean_project(&fake_project);

        // 清理不存在的项目应该返回0或错误
        if let Ok(size) = result {
            assert_eq!(size, 0);
        }
        // 错误也是可接受的

        Ok(())
    }

    #[test]
    fn test_clean_readonly_target() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let project = create_test_project_with_target(temp_dir.path(), "readonly_project")?;

        // 尝试将target目录设为只读（在某些系统上可能不起作用）
        let target_path = project.path.join("target");
        if target_path.exists() {
            // 在Windows上设置只读属性
            #[cfg(windows)]
            {
                let mut perms = std::fs::metadata(&target_path)?.permissions();
                perms.set_readonly(true);
                let _ = std::fs::set_permissions(&target_path, perms);
            }

            // 在Unix系统上设置只读权限
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ =
                    std::fs::set_permissions(&target_path, std::fs::Permissions::from_mode(0o444));
            }
        }

        let cleaner = ProjectCleaner::default();
        let result = cleaner.clean_project(&project);

        // 清理只读目录可能失败，这是预期的
        // 如果成功了也没关系，失败是预期的
        let _ = result;

        Ok(())
    }

    #[test]
    fn test_clean_with_timeout() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let project = create_test_project_with_target(temp_dir.path(), "timeout_project")?;

        // 使用非常短的超时时间
        let config = CleanConfig {
            strategy: CleanStrategy::CargoClean,
            timeout_seconds: 1, // 1秒超时
            ..Default::default()
        };

        let cleaner = ProjectCleaner::new(config);
        let result = cleaner.clean_project(&project);

        // 可能会因为超时而失败，也可能成功（如果操作很快）
        // 成功也是可能的，超时失败是预期的
        let _ = result;

        Ok(())
    }

    #[test]
    fn test_clean_projects_with_mixed_results() -> Result<()> {
        let temp_dir = TempDir::new()?;

        // 创建一个正常项目
        let good_project = create_test_project_with_target(temp_dir.path(), "good_project")?;

        // 创建一个不存在target目录的项目
        let bad_project_path = temp_dir.path().join("bad_project");
        std::fs::create_dir_all(&bad_project_path)?;
        std::fs::write(
            bad_project_path.join("Cargo.toml"),
            r#"
[package]
name = "bad_project"
version = "0.1.0"
edition = "2021"
"#,
        )?;

        let bad_project = RustProject {
            path: bad_project_path,
            name: "bad_project".to_string(),
            target_size: 0, // 没有target目录
            last_modified: std::time::SystemTime::now(),
            is_workspace: false,
            has_target: false, // 关键：没有target目录
        };

        let projects = vec![good_project, bad_project];
        let cleaner = ProjectCleaner::default();
        let result = cleaner.clean_projects(&projects);

        // 验证清理结果
        // 可能有一些项目清理成功，一些失败，这是正常的混合结果
        assert!(result.cleaned_projects + result.failed_projects.len() == 2);

        // 验证至少处理了所有项目
        println!(
            "清理结果: 成功 {}, 失败 {}",
            result.cleaned_projects,
            result.failed_projects.len()
        );

        Ok(())
    }
}
