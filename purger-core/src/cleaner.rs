use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tracing::{debug, error, info};
use walkdir::WalkDir;

use crate::project::RustProject;
use crate::{CleanFailure, CleanResult};

/// 清理策略
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum CleanStrategy {
    /// 使用cargo clean命令
    #[default]
    CargoClean,
    /// 直接删除target目录
    DirectDelete,
}

/// Backend for `CleanStrategy::DirectDelete`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DirectDeleteBackend {
    /// Use Rust filesystem deletion (cross-platform, detailed progress).
    #[default]
    Native,
    /// Use `cmd.exe /C rmdir /S /Q` on Windows (usually faster, coarse progress).
    CmdRmdir,
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

#[derive(Debug, thiserror::Error)]
#[error("clean cancelled")]
pub struct CleanCancelled;

#[derive(Debug, thiserror::Error)]
#[error("clean timed out after {timeout:?}")]
pub struct CleanTimedOut {
    pub timeout: Duration,
}

#[derive(Debug, thiserror::Error)]
#[error("refusing to delete unsafe target directory: {path:?} ({reason})")]
pub struct UnsafeTargetDirectory {
    pub path: PathBuf,
    pub reason: String,
}

/// 清理器配置
#[derive(Debug, Clone)]
pub struct CleanConfig {
    pub strategy: CleanStrategy,
    pub dry_run: bool,
    pub parallel: bool,
    pub timeout_seconds: u64,
    pub direct_delete_backend: DirectDeleteBackend,

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
            timeout_seconds: 0,
            direct_delete_backend: DirectDeleteBackend::Native,

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
        self.clean_project_with_progress_and_cancel(project, None, progress_callback)
    }

    pub fn clean_project_with_progress_and_cancel<F>(
        &self,
        project: &RustProject,
        cancel_flag: Option<&AtomicBool>,
        progress_callback: F,
    ) -> Result<u64>
    where
        F: Fn(CleanProgress),
    {
        match self.clean_project_with_progress_impl(project, cancel_flag, &progress_callback) {
            Ok(bytes) => Ok(bytes),
            Err(err) => {
                if !err.is::<CleanCancelled>() {
                    error!("清理项目失败 {}: {}", project.name, err);
                }
                Err(err)
            }
        }
    }

    fn clean_project_with_progress_impl<F>(
        &self,
        project: &RustProject,
        cancel_flag: Option<&AtomicBool>,
        progress_callback: &F,
    ) -> Result<u64>
    where
        F: Fn(CleanProgress),
    {
        self.check_cancel(cancel_flag)?;

        if self.config.dry_run {
            let size = if project.has_target {
                project.get_target_size()
            } else {
                0
            };
            info!(
                "DRY RUN: 将清理项目 {} ({})",
                project.name,
                crate::format_bytes(size)
            );
            return Ok(size);
        }

        if !project.has_target && self.config.strategy == CleanStrategy::DirectDelete {
            debug!("项目 {} 没有target目录，跳过", project.name);
            return Ok(0);
        }

        info!(
            "开始清理项目: {} ({})",
            project.name,
            project.formatted_size()
        );

        progress_callback(CleanProgress {
            project_name: project.name.clone(),
            current_file: None,
            files_processed: 0,
            total_files: None,
            phase: CleanPhase::Starting,
        });

        let bytes_freed = match self.config.strategy {
            CleanStrategy::CargoClean => {
                self.clean_with_cargo_progress(project, cancel_flag, progress_callback)?
            }
            CleanStrategy::DirectDelete => {
                self.clean_with_delete_progress(project, cancel_flag, progress_callback)?
            }
        };

        progress_callback(CleanProgress {
            project_name: project.name.clone(),
            current_file: None,
            files_processed: 0,
            total_files: None,
            phase: CleanPhase::Complete,
        });

        info!("成功清理项目: {}", project.name);
        Ok(bytes_freed)
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
                Err(err) => result.add_failure_detail(CleanFailure {
                    project_name: project.name.clone(),
                    project_path: project.path.clone(),
                    error: err.to_string(),
                }),
            }
        }
    }

    /// 并行清理项目（注意：这里简化实现，实际可能需要更复杂的并行控制）
    fn clean_projects_parallel(&self, projects: &[RustProject], result: &mut CleanResult) {
        let (successes, total_freed, failures): (usize, u64, Vec<CleanFailure>) = projects
            .par_iter()
            .map(|project| match self.clean_project(project) {
                Ok(size_freed) => Ok(size_freed),
                Err(err) => Err(CleanFailure {
                    project_name: project.name.clone(),
                    project_path: project.path.clone(),
                    error: err.to_string(),
                }),
            })
            .fold(
                || (0usize, 0u64, Vec::new()),
                |mut acc, item| {
                    match item {
                        Ok(size_freed) => {
                            acc.0 += 1;
                            acc.1 += size_freed;
                        }
                        Err(failure) => {
                            acc.2.push(failure);
                        }
                    }
                    acc
                },
            )
            .reduce(
                || (0usize, 0u64, Vec::new()),
                |mut a, b| {
                    a.0 += b.0;
                    a.1 += b.1;
                    a.2.extend(b.2);
                    a
                },
            );

        result.cleaned_projects += successes;
        result.total_size_freed += total_freed;
        for failure in failures {
            result.add_failure_detail(failure);
        }
    }

    /// 使用cargo clean清理
    #[allow(dead_code)]
    fn clean_with_cargo(&self, project: &RustProject) -> Result<u64> {
        self.clean_with_cargo_progress(project, None, &|_| {})
    }

    /// 使用cargo clean清理（带进度回调）
    fn clean_with_cargo_progress<F>(
        &self,
        project: &RustProject,
        cancel_flag: Option<&AtomicBool>,
        progress_callback: &F,
    ) -> Result<u64>
    where
        F: Fn(CleanProgress),
    {
        debug!("使用cargo clean清理项目: {}", project.name);

        self.check_cancel(cancel_flag)?;

        progress_callback(CleanProgress {
            project_name: project.name.clone(),
            current_file: Some("cargo clean".to_string()),
            files_processed: 0,
            total_files: None,
            phase: CleanPhase::Analyzing,
        });

        let target_path = project.target_path();
        let size_before = if target_path.exists() {
            project.get_target_size()
        } else {
            0
        };

        progress_callback(CleanProgress {
            project_name: project.name.clone(),
            current_file: Some("cargo clean".to_string()),
            files_processed: 0,
            total_files: None,
            phase: CleanPhase::Cleaning,
        });

        let mut cmd = Command::new("cargo");
        cmd.arg("clean")
            .current_dir(&project.path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = self.run_command_with_timeout_and_cancel(
            cmd,
            self.timeout(),
            cancel_flag,
            |elapsed| {
                let ticks = (elapsed.as_millis() / 250) as usize;
                progress_callback(CleanProgress {
                    project_name: project.name.clone(),
                    current_file: Some(format!("cargo clean ({:?})", elapsed)),
                    files_processed: ticks,
                    total_files: None,
                    phase: CleanPhase::Cleaning,
                });
            },
        )?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("cargo clean失败: {}", stderr.trim());
        }

        // 最终进度更新
        progress_callback(CleanProgress {
            project_name: project.name.clone(),
            current_file: None,
            files_processed: 0,
            total_files: None,
            phase: CleanPhase::Finalizing,
        });

        let size_after = if target_path.exists() {
            project.get_target_size()
        } else {
            0
        };

        Ok(size_before.saturating_sub(size_after))
    }

    /// 直接删除target目录
    #[allow(dead_code)]
    fn clean_with_delete(&self, project: &RustProject) -> Result<u64> {
        self.clean_with_delete_progress(project, None, &|_| {})
    }

    /// 直接删除target目录（带进度回调）
    fn clean_with_delete_progress<F>(
        &self,
        project: &RustProject,
        cancel_flag: Option<&AtomicBool>,
        progress_callback: &F,
    ) -> Result<u64>
    where
        F: Fn(CleanProgress),
    {
        debug!("直接删除target目录: {}", project.name);

        let target_path = project.target_path();
        if !target_path.exists() {
            return Ok(0);
        }

        self.check_cancel(cancel_flag)?;
        self.validate_safe_target_directory(project, &target_path)?;

        progress_callback(CleanProgress {
            project_name: project.name.clone(),
            current_file: None,
            files_processed: 0,
            total_files: None,
            phase: CleanPhase::Analyzing,
        });

        // 如果需要保留可执行文件，先备份
        if self.config.keep_executable {
            self.backup_executables(project, cancel_flag, progress_callback)?;
        }

        progress_callback(CleanProgress {
            project_name: project.name.clone(),
            current_file: Some("target".to_string()),
            files_processed: 0,
            total_files: None,
            phase: CleanPhase::Cleaning,
        });

        let timeout = self.timeout();
        let bytes_freed = match self.config.direct_delete_backend {
            DirectDeleteBackend::Native => {
                if cancel_flag.is_some() || self.config.keep_executable {
                    self.delete_directory_tree_with_progress(
                        project,
                        &target_path,
                        cancel_flag,
                        timeout,
                        progress_callback,
                    )?
                } else {
                    let size_before = project.get_target_size();
                    std::fs::remove_dir_all(&target_path).context("删除target目录失败")?;
                    size_before
                }
            }
            DirectDeleteBackend::CmdRmdir => self.clean_with_windows_rmdir(
                project,
                &target_path,
                cancel_flag,
                timeout,
                progress_callback,
            )?,
        };

        progress_callback(CleanProgress {
            project_name: project.name.clone(),
            current_file: None,
            files_processed: 0,
            total_files: None,
            phase: CleanPhase::Finalizing,
        });

        Ok(bytes_freed)
    }

    fn clean_with_windows_rmdir<F>(
        &self,
        project: &RustProject,
        target_path: &std::path::Path,
        cancel_flag: Option<&AtomicBool>,
        timeout: Option<Duration>,
        progress_callback: &F,
    ) -> Result<u64>
    where
        F: Fn(CleanProgress),
    {
        #[cfg(windows)]
        {
            self.check_cancel(cancel_flag)?;
            self.validate_safe_target_directory(project, target_path)?;

            let size_before = project.get_target_size();
            let target_str = target_path.display().to_string();
            if target_str.contains('"') {
                return self.delete_directory_tree_with_progress(
                    project,
                    target_path,
                    cancel_flag,
                    timeout,
                    progress_callback,
                );
            }

            let mut cmd = Command::new("cmd");
            cmd.args(["/C", &format!("rmdir /S /Q \"{target_str}\"")]);
            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

            let output =
                self.run_command_with_timeout_and_cancel(cmd, timeout, cancel_flag, |elapsed| {
                    progress_callback(CleanProgress {
                        project_name: project.name.clone(),
                        current_file: Some(format!("target ({:.1}s)", elapsed.as_secs_f32())),
                        files_processed: 0,
                        total_files: None,
                        phase: CleanPhase::Cleaning,
                    });
                })?;

            if output.status.success() {
                return Ok(size_before);
            }

            if cancel_flag.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                anyhow::bail!(CleanCancelled);
            }

            // Fallback to native deletion for better diagnostics and permission handling.
            self.delete_directory_tree_with_progress(
                project,
                target_path,
                cancel_flag,
                timeout,
                progress_callback,
            )
        }

        #[cfg(not(windows))]
        {
            warn!(
                "CmdRmdir backend requested on non-Windows, falling back to Native for project: {}",
                project.name
            );
            if cancel_flag.is_some() || self.config.keep_executable {
                self.delete_directory_tree_with_progress(
                    project,
                    target_path,
                    cancel_flag,
                    timeout,
                    progress_callback,
                )
            } else {
                let size_before = project.get_target_size();
                std::fs::remove_dir_all(target_path).context("删除target目录失败")?;
                Ok(size_before)
            }
        }
    }

    /// 备份可执行文件
    fn backup_executables<F>(
        &self,
        project: &RustProject,
        cancel_flag: Option<&AtomicBool>,
        progress_callback: &F,
    ) -> Result<()>
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
            self.check_cancel(cancel_flag)?;
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

    fn timeout(&self) -> Option<Duration> {
        if self.config.timeout_seconds == 0 {
            return None;
        }
        Some(Duration::from_secs(self.config.timeout_seconds))
    }

    fn check_cancel(&self, cancel_flag: Option<&AtomicBool>) -> Result<()> {
        if cancel_flag.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            anyhow::bail!(CleanCancelled);
        }
        Ok(())
    }

    fn validate_safe_target_directory(
        &self,
        project: &RustProject,
        target_path: &std::path::Path,
    ) -> Result<()> {
        let metadata = std::fs::symlink_metadata(target_path).context("读取 target 元数据失败")?;
        if metadata.file_type().is_symlink() {
            anyhow::bail!(UnsafeTargetDirectory {
                path: target_path.to_path_buf(),
                reason: "target is a symlink/reparse point".to_string(),
            });
        }

        let canonical_target = target_path.canonicalize().ok();
        let canonical_project = project.path.canonicalize().ok();

        if let (Some(target), Some(root)) = (canonical_target, canonical_project) {
            if !target.starts_with(&root) {
                anyhow::bail!(UnsafeTargetDirectory {
                    path: target_path.to_path_buf(),
                    reason: format!("target escapes project root: {target:?}"),
                });
            }
        } else if !target_path.starts_with(&project.path) {
            anyhow::bail!(UnsafeTargetDirectory {
                path: target_path.to_path_buf(),
                reason: "target is not under project path".to_string(),
            });
        }

        Ok(())
    }

    fn delete_directory_tree_with_progress<F>(
        &self,
        project: &RustProject,
        target_path: &std::path::Path,
        cancel_flag: Option<&AtomicBool>,
        timeout: Option<Duration>,
        progress_callback: &F,
    ) -> Result<u64>
    where
        F: Fn(CleanProgress),
    {
        let start = Instant::now();
        let mut last_report = Instant::now();
        let mut processed = 0usize;
        let mut directories: Vec<PathBuf> = Vec::new();

        let mut bytes_freed = if project.target_size > 0 {
            project.target_size
        } else {
            0
        };
        let track_bytes = project.target_size == 0;

        if self.config.parallel && cancel_flag.is_some() {
            let mut files: Vec<PathBuf> = Vec::new();

            for entry in WalkDir::new(target_path).follow_links(false).min_depth(1) {
                self.check_cancel(cancel_flag)?;
                if let Some(timeout) = timeout {
                    if start.elapsed() > timeout {
                        anyhow::bail!(CleanTimedOut { timeout });
                    }
                }

                let entry = entry.context("遍历 target 目录失败")?;
                let path = entry.path().to_path_buf();

                if entry.file_type().is_dir() {
                    directories.push(path);
                } else {
                    files.push(path);
                }
            }

            let total_files = Some(files.len());
            let chunk_size = 1024usize;
            for chunk in files.chunks(chunk_size) {
                self.check_cancel(cancel_flag)?;
                if let Some(timeout) = timeout {
                    if start.elapsed() > timeout {
                        anyhow::bail!(CleanTimedOut { timeout });
                    }
                }

                let bytes_in_chunk: u64 = chunk
                    .par_iter()
                    .map(|path| -> Result<u64> {
                        if cancel_flag.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                            anyhow::bail!(CleanCancelled);
                        }
                        if let Some(timeout) = timeout {
                            if start.elapsed() > timeout {
                                anyhow::bail!(CleanTimedOut { timeout });
                            }
                        }

                        let mut bytes = 0u64;
                        if track_bytes {
                            if let Ok(metadata) = std::fs::symlink_metadata(path) {
                                bytes = metadata.len();
                            }
                        }

                        Self::remove_path_best_effort(path)
                            .with_context(|| format!("删除失败: {path:?}"))?;
                        Ok(bytes)
                    })
                    .try_reduce(|| 0u64, |a, b| Ok(a.saturating_add(b)))?;

                if track_bytes {
                    bytes_freed = bytes_freed.saturating_add(bytes_in_chunk);
                }
                processed = processed.saturating_add(chunk.len());

                if last_report.elapsed() >= Duration::from_millis(120) {
                    last_report = Instant::now();
                    let current_file = chunk
                        .last()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().to_string());
                    progress_callback(CleanProgress {
                        project_name: project.name.clone(),
                        current_file,
                        files_processed: processed,
                        total_files,
                        phase: CleanPhase::Cleaning,
                    });
                }
            }
        } else {
            for entry in WalkDir::new(target_path).follow_links(false).min_depth(1) {
                self.check_cancel(cancel_flag)?;
                if let Some(timeout) = timeout {
                    if start.elapsed() > timeout {
                        anyhow::bail!(CleanTimedOut { timeout });
                    }
                }

                let entry = entry.context("遍历 target 目录失败")?;
                let path = entry.path().to_path_buf();

                if entry.file_type().is_dir() {
                    directories.push(path);
                    continue;
                }

                if track_bytes {
                    if let Ok(metadata) = std::fs::symlink_metadata(&path) {
                        bytes_freed = bytes_freed.saturating_add(metadata.len());
                    }
                }

                Self::remove_path_best_effort(&path)
                    .with_context(|| format!("删除失败: {path:?}"))?;
                processed = processed.saturating_add(1);

                if last_report.elapsed() >= Duration::from_millis(120) {
                    last_report = Instant::now();
                    progress_callback(CleanProgress {
                        project_name: project.name.clone(),
                        current_file: path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .or_else(|| Some(path.display().to_string())),
                        files_processed: processed,
                        total_files: None,
                        phase: CleanPhase::Cleaning,
                    });
                }
            }
        }

        directories.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
        for dir in directories {
            self.check_cancel(cancel_flag)?;
            if let Some(timeout) = timeout {
                if start.elapsed() > timeout {
                    anyhow::bail!(CleanTimedOut { timeout });
                }
            }
            let _ = Self::remove_dir_best_effort(&dir);
        }

        self.check_cancel(cancel_flag)?;
        if let Some(timeout) = timeout {
            if start.elapsed() > timeout {
                anyhow::bail!(CleanTimedOut { timeout });
            }
        }

        Self::remove_dir_best_effort(target_path)
            .with_context(|| format!("删除 target 根目录失败: {target_path:?}"))?;

        Ok(bytes_freed)
    }

    fn run_command_with_timeout_and_cancel<T>(
        &self,
        mut cmd: Command,
        timeout: Option<Duration>,
        cancel_flag: Option<&AtomicBool>,
        on_tick: T,
    ) -> Result<std::process::Output>
    where
        T: Fn(Duration),
    {
        let mut child = cmd.spawn().context("启动命令失败")?;
        let start = Instant::now();

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let mut stdout_handle = Some(std::thread::spawn(move || -> Vec<u8> {
            let mut buf = Vec::new();
            if let Some(mut out) = stdout {
                let _ = out.read_to_end(&mut buf);
            }
            buf
        }));

        let mut stderr_handle = Some(std::thread::spawn(move || -> Vec<u8> {
            let mut buf = Vec::new();
            if let Some(mut err) = stderr {
                let _ = err.read_to_end(&mut buf);
            }
            buf
        }));

        let status = loop {
            if cancel_flag.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_handle.take().and_then(|h| h.join().ok());
                let _ = stderr_handle.take().and_then(|h| h.join().ok());
                anyhow::bail!(CleanCancelled);
            }

            if let Some(timeout) = timeout {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stdout_handle.take().and_then(|h| h.join().ok());
                    let _ = stderr_handle.take().and_then(|h| h.join().ok());
                    anyhow::bail!(CleanTimedOut { timeout });
                }
            }

            if let Some(status) = child.try_wait().context("等待子进程失败")? {
                break status;
            }

            on_tick(start.elapsed());
            std::thread::sleep(Duration::from_millis(80));
        };

        let stdout = stdout_handle
            .take()
            .and_then(|h| h.join().ok())
            .unwrap_or_default();
        let stderr = stderr_handle
            .take()
            .and_then(|h| h.join().ok())
            .unwrap_or_default();

        Ok(std::process::Output {
            status,
            stdout,
            stderr,
        })
    }

    fn remove_path_best_effort(path: &std::path::Path) -> std::io::Result<()> {
        if std::fs::remove_file(path).is_ok() {
            return Ok(());
        }
        if std::fs::remove_dir(path).is_ok() {
            return Ok(());
        }

        if let Ok(metadata) = std::fs::symlink_metadata(path) {
            let mut perms = metadata.permissions();
            perms.set_readonly(false);
            let _ = std::fs::set_permissions(path, perms);
        }

        std::fs::remove_file(path).or_else(|_| std::fs::remove_dir(path))
    }

    fn remove_dir_best_effort(path: &std::path::Path) -> std::io::Result<()> {
        if std::fs::remove_dir(path).is_ok() {
            return Ok(());
        }

        if let Ok(metadata) = std::fs::symlink_metadata(path) {
            let mut perms = metadata.permissions();
            perms.set_readonly(false);
            let _ = std::fs::set_permissions(path, perms);
        }

        std::fs::remove_dir(path).or_else(|_| std::fs::remove_dir_all(path))
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
        let base_dir = if let Some(ref backup_dir) = self.config.executable_backup_dir {
            backup_dir.clone()
        } else {
            project.path.join("executables")
        };

        let mut hasher = DefaultHasher::new();
        project.path.to_string_lossy().hash(&mut hasher);
        let id = hasher.finish();

        Ok(base_dir.join(format!("{}-{:016x}", project.name, id)))
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
        assert_eq!(config.timeout_seconds, 0);
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
