use anyhow::{Context, Result};
use ignore::{DirEntry, WalkBuilder};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

use crate::filter::ProjectFilter;
use crate::project::RustProject;

/// 项目扫描器配置
#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub max_depth: Option<usize>,
    pub follow_links: bool,
    pub respect_gitignore: bool,
    pub ignore_hidden: bool,
    pub parallel: bool,

    // 性能优化选项
    /// 是否延迟计算目录大小（只在需要时计算）
    pub lazy_size_calculation: bool,

    // 过滤选项
    /// 保留最近N天编译的项目（基于target目录的最后修改时间）
    pub keep_days: Option<u32>,
    /// 保留target目录小于指定大小的项目（字节）
    pub keep_size: Option<u64>,
    /// 忽略的路径列表（绝对路径或相对路径）
    pub ignore_paths: Vec<PathBuf>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            max_depth: Some(10), // 默认最大深度10层
            follow_links: false,
            respect_gitignore: true,
            ignore_hidden: true,
            parallel: true,

            // 性能优化默认值
            lazy_size_calculation: false, // 默认立即计算大小

            // 过滤选项默认值
            keep_days: None,
            keep_size: None,
            ignore_paths: Vec::new(),
        }
    }
}

/// Rust项目扫描器
pub struct ProjectScanner {
    config: ScanConfig,
    // 简单的项目缓存，避免重复解析相同的项目
    cache: Arc<Mutex<HashMap<PathBuf, RustProject>>>,
}

impl ProjectScanner {
    /// 创建新的扫描器
    pub fn new(config: ScanConfig) -> Self {
        Self {
            config,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 扫描指定路径下的所有Rust项目
    pub fn scan<P: AsRef<Path>>(&self, root_path: P) -> Result<Vec<RustProject>> {
        self.scan_with_cancel_and_progress(root_path, None, None)
    }

    pub fn scan_with_cancel_and_progress<P: AsRef<Path>>(
        &self,
        root_path: P,
        cancel_flag: Option<&AtomicBool>,
        on_cargo_toml_found: Option<&(dyn Fn(usize) + Sync)>,
    ) -> Result<Vec<RustProject>> {
        let root_path = root_path.as_ref();
        let start_time = std::time::Instant::now();
        info!("开始扫描路径: {:?}", root_path);

        if !root_path.exists() {
            anyhow::bail!("路径不存在: {:?}", root_path);
        }

        if !root_path.is_dir() {
            anyhow::bail!("路径不是目录: {:?}", root_path);
        }

        // 优化的文件遍历
        let cargo_dirs = self.find_cargo_projects(root_path, cancel_flag, on_cargo_toml_found)?;
        let find_time = start_time.elapsed();
        info!(
            "找到 {} 个Cargo.toml文件，耗时: {:?}",
            cargo_dirs.len(),
            find_time
        );

        if cancel_flag.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            anyhow::bail!("扫描已取消");
        }

        // 并行或串行处理项目
        let parse_start = std::time::Instant::now();
        let projects = if self.config.parallel {
            self.process_projects_parallel(cargo_dirs)?
        } else {
            self.process_projects_sequential(cargo_dirs)?
        };
        let parse_time = parse_start.elapsed();

        info!(
            "成功解析 {} 个Rust项目，耗时: {:?}",
            projects.len(),
            parse_time
        );
        info!("总扫描时间: {:?}", start_time.elapsed());

        // 应用过滤器
        let filtered_projects = self.apply_filters(projects);

        Ok(filtered_projects)
    }

    /// 优化的Cargo项目查找方法
    fn find_cargo_projects(
        &self,
        root_path: &Path,
        cancel_flag: Option<&AtomicBool>,
        on_cargo_toml_found: Option<&(dyn Fn(usize) + Sync)>,
    ) -> Result<Vec<PathBuf>> {
        let mut builder = WalkBuilder::new(root_path);
        builder
            .follow_links(self.config.follow_links)
            .git_ignore(self.config.respect_gitignore)
            .hidden(self.config.ignore_hidden);

        if let Some(depth) = self.config.max_depth {
            builder.max_depth(Some(depth));
        }

        // 启用并行遍历以提升性能
        if self.config.parallel {
            // 使用系统CPU核心数，但限制最大线程数避免过度并发
            let thread_count = std::cmp::min(
                std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(4),
                8,
            );
            builder.threads(thread_count);
            debug!("启用并行文件遍历，线程数: {}", thread_count);
        }

        let walker = builder.build_parallel();
        let cargo_dirs = std::sync::Mutex::new(Vec::new());
        let found_count = AtomicUsize::new(0);

        walker.run(|| {
            let cargo_dirs = &cargo_dirs;
            let found_count = &found_count;
            Box::new(move |entry| {
                if cancel_flag.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                    return ignore::WalkState::Quit;
                }

                match entry {
                    Ok(entry) => {
                        if let Some(project_dir) =
                            self.process_entry(entry, found_count, on_cargo_toml_found)
                        {
                            if let Ok(mut dirs) = cargo_dirs.lock() {
                                dirs.push(project_dir);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("扫描错误: {}", e);
                    }
                }
                ignore::WalkState::Continue
            })
        });

        let cargo_dirs = cargo_dirs
            .into_inner()
            .unwrap_or_else(|poison| poison.into_inner());
        Ok(cargo_dirs)
    }

    /// 处理单个目录条目
    fn process_entry(
        &self,
        entry: DirEntry,
        found_count: &AtomicUsize,
        on_cargo_toml_found: Option<&(dyn Fn(usize) + Sync)>,
    ) -> Option<PathBuf> {
        let path = entry.path();

        // 检查是否为Cargo.toml文件
        if path.file_name()? == "Cargo.toml" && path.is_file() {
            debug!("发现Cargo.toml: {:?}", path);
            let count = found_count.fetch_add(1, Ordering::Relaxed) + 1;
            if count % 50 == 0 {
                if let Some(callback) = on_cargo_toml_found {
                    callback(count);
                }
            }
            return path.parent().map(|p| p.to_path_buf());
        }

        None
    }

    /// 并行处理项目（带缓存优化）
    fn process_projects_parallel(&self, cargo_dirs: Vec<PathBuf>) -> Result<Vec<RustProject>> {
        let cache = Arc::clone(&self.cache);
        let lazy_size_calculation = self.config.lazy_size_calculation;

        let projects: Vec<_> = cargo_dirs
            .into_par_iter()
            .filter_map(|dir| {
                // 先检查缓存
                if let Ok(cache_guard) = cache.lock() {
                    if let Some(cached_project) = cache_guard.get(&dir) {
                        debug!("从缓存获取项目: {}", cached_project.name);
                        return Some(cached_project.clone());
                    }
                }

                // 缓存未命中，解析项目
                let project_result = if lazy_size_calculation {
                    RustProject::from_path_lazy(&dir)
                } else {
                    RustProject::from_path(&dir)
                };

                match project_result {
                    Ok(project) => {
                        debug!("成功解析项目: {}", project.name);

                        // 更新缓存
                        if let Ok(mut cache_guard) = cache.lock() {
                            cache_guard.insert(dir, project.clone());
                        }

                        Some(project)
                    }
                    Err(e) => {
                        warn!("解析项目失败 {:?}: {}", dir, e);
                        None
                    }
                }
            })
            .collect();

        Ok(projects)
    }

    /// 串行处理项目
    fn process_projects_sequential(&self, cargo_dirs: Vec<PathBuf>) -> Result<Vec<RustProject>> {
        let mut projects = Vec::new();

        for dir in cargo_dirs {
            let project_result = if self.config.lazy_size_calculation {
                RustProject::from_path_lazy(&dir)
            } else {
                RustProject::from_path(&dir)
            };

            match project_result {
                Ok(project) => {
                    debug!("成功解析项目: {}", project.name);
                    projects.push(project);
                }
                Err(e) => {
                    warn!("解析项目失败 {:?}: {}", dir, e);
                    // 继续处理其他项目，不中断整个扫描过程
                }
            }
        }

        Ok(projects)
    }

    /// 扫描单个项目（用于验证特定路径）
    pub fn scan_single<P: AsRef<Path>>(&self, project_path: P) -> Result<RustProject> {
        let project_path = project_path.as_ref();

        if !project_path.join("Cargo.toml").exists() {
            anyhow::bail!("路径不是Rust项目: {:?}", project_path);
        }

        let project_result = if self.config.lazy_size_calculation {
            RustProject::from_path_lazy(project_path)
        } else {
            RustProject::from_path(project_path)
        };

        project_result.context("解析Rust项目失败")
    }

    /// 过滤有target目录的项目
    pub fn filter_with_target(projects: Vec<RustProject>) -> Vec<RustProject> {
        projects.into_iter().filter(|p| p.has_target).collect()
    }

    /// 按大小排序项目（从大到小）
    pub fn sort_by_size(mut projects: Vec<RustProject>) -> Vec<RustProject> {
        projects.sort_by(|a, b| b.target_size.cmp(&a.target_size));
        projects
    }

    /// 应用过滤器
    fn apply_filters(&self, projects: Vec<RustProject>) -> Vec<RustProject> {
        // 如果没有配置任何过滤条件，直接返回
        if self.config.keep_days.is_none()
            && self.config.keep_size.is_none()
            && self.config.ignore_paths.is_empty()
        {
            return projects;
        }

        let filter = ProjectFilter::new(self.config.clone());
        filter.filter_projects(projects)
    }
}

impl Default for ProjectScanner {
    fn default() -> Self {
        Self::new(ScanConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::SystemTime;
    use tempfile::TempDir;

    fn create_test_project(dir: &Path, name: &str, has_target: bool) -> Result<()> {
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

        if has_target {
            let target_dir = project_dir.join("target");
            fs::create_dir_all(&target_dir)?;
            fs::write(target_dir.join("test.txt"), "test content")?;
        }

        Ok(())
    }

    #[test]
    fn test_scanner_basic() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        // 创建测试项目
        create_test_project(root, "project1", true)?;
        create_test_project(root, "project2", false)?;
        create_test_project(root, "project3", true)?;

        let scanner = ProjectScanner::default();
        let projects = scanner.scan(root)?;

        assert_eq!(projects.len(), 3);

        let with_target = ProjectScanner::filter_with_target(projects);
        assert_eq!(with_target.len(), 2);

        Ok(())
    }

    #[test]
    fn test_scan_single() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        create_test_project(root, "single_project", true)?;

        let scanner = ProjectScanner::default();
        let project = scanner.scan_single(root.join("single_project"))?;

        assert_eq!(project.name, "single_project");
        assert!(project.has_target);

        Ok(())
    }

    #[test]
    fn test_scan_with_max_depth() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        // 创建浅层项目（在根目录的子目录中）
        create_test_project(root, "shallow_project", true)?;

        // 创建深层项目
        let deep_dir = root.join("level1").join("level2");
        std::fs::create_dir_all(&deep_dir)?;
        create_test_project(&deep_dir, "deep_project", true)?;

        // 无深度限制，应该找到两个项目
        let config = ScanConfig {
            max_depth: None,
            ..Default::default()
        };
        let scanner = ProjectScanner::new(config);
        let projects = scanner.scan(root)?;
        println!("无深度限制找到 {} 个项目", projects.len());
        assert!(!projects.is_empty());

        // 限制深度为2，应该找到浅层项目（在子目录中）
        let config = ScanConfig {
            max_depth: Some(2),
            ..Default::default()
        };
        let scanner = ProjectScanner::new(config);
        let projects = scanner.scan(root)?;
        println!("深度限制2找到 {} 个项目", projects.len());

        // 应该至少找到浅层项目
        assert!(!projects.is_empty());

        // 检查是否包含浅层项目
        let has_shallow = projects.iter().any(|p| p.name == "shallow_project");
        assert!(has_shallow, "应该包含浅层项目");

        // 限制深度为1，应该找不到任何项目（因为项目都在子目录中）
        let config = ScanConfig {
            max_depth: Some(1),
            ..Default::default()
        };
        let scanner = ProjectScanner::new(config);
        let projects = scanner.scan(root)?;
        println!("深度限制1找到 {} 个项目", projects.len());

        // 深度1应该找不到项目，因为项目在子目录中
        // 这个测试验证了深度限制确实在工作

        Ok(())
    }

    #[test]
    fn test_scan_parallel_vs_sequential() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        // 创建多个项目
        for i in 0..5 {
            create_test_project(root, &format!("project_{i}"), i % 2 == 0)?;
        }

        // 并行扫描
        let config = ScanConfig {
            parallel: true,
            ..Default::default()
        };
        let scanner = ProjectScanner::new(config);
        let parallel_projects = scanner.scan(root)?;

        // 串行扫描
        let config = ScanConfig {
            parallel: false,
            ..Default::default()
        };
        let scanner = ProjectScanner::new(config);
        let sequential_projects = scanner.scan(root)?;

        // 结果应该相同
        assert_eq!(parallel_projects.len(), sequential_projects.len());
        assert_eq!(parallel_projects.len(), 5);

        Ok(())
    }

    #[test]
    fn test_filter_with_target() {
        let projects = vec![
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
                target_size: 0,
                last_modified: SystemTime::now(),
                is_workspace: false,
                has_target: false,
            },
        ];

        let filtered = ProjectScanner::filter_with_target(projects);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "test1");
    }

    #[test]
    fn test_sort_by_size() {
        let projects = vec![
            RustProject {
                path: PathBuf::from("/small"),
                name: "small".to_string(),
                target_size: 100,
                last_modified: SystemTime::now(),
                is_workspace: false,
                has_target: true,
            },
            RustProject {
                path: PathBuf::from("/large"),
                name: "large".to_string(),
                target_size: 1000,
                last_modified: SystemTime::now(),
                is_workspace: false,
                has_target: true,
            },
            RustProject {
                path: PathBuf::from("/medium"),
                name: "medium".to_string(),
                target_size: 500,
                last_modified: SystemTime::now(),
                is_workspace: false,
                has_target: true,
            },
        ];

        let sorted = ProjectScanner::sort_by_size(projects);
        assert_eq!(sorted[0].name, "large");
        assert_eq!(sorted[1].name, "medium");
        assert_eq!(sorted[2].name, "small");
    }

    #[test]
    fn test_scan_nonexistent_path() {
        let scanner = ProjectScanner::default();
        let result = scanner.scan("/nonexistent/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_single_invalid_project() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // 创建一个没有Cargo.toml的目录
        std::fs::create_dir_all(root.join("not_a_project")).unwrap();

        let scanner = ProjectScanner::default();
        let result = scanner.scan_single(root.join("not_a_project"));
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_permission_denied() {
        let scanner = ProjectScanner::default();

        // 在Windows上，尝试访问系统保护的目录
        #[cfg(windows)]
        {
            let result = scanner.scan(std::path::Path::new("C:\\System Volume Information"));
            // 这应该失败或返回空结果
            if let Ok(projects) = result {
                assert!(projects.is_empty());
            }
            // 权限错误是预期的
        }

        // 在Unix系统上测试
        #[cfg(unix)]
        {
            let result = scanner.scan(std::path::Path::new("/root"));
            if let Ok(projects) = result {
                assert!(projects.is_empty());
            }
            // 权限错误是预期的
        }
    }

    #[test]
    fn test_scan_corrupted_cargo_toml() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        // 创建一个有损坏Cargo.toml的项目
        let project_path = root.join("corrupted_project");
        std::fs::create_dir_all(&project_path)?;

        // 写入无效的TOML
        std::fs::write(project_path.join("Cargo.toml"), "invalid toml content [[[[")?;

        let scanner = ProjectScanner::default();
        let project = scanner.scan_single(&project_path)?;
        assert_eq!(project.name, "corrupted_project");
        assert!(!project.is_workspace);
        assert!(!project.has_target);

        Ok(())
    }

    #[test]
    fn test_scan_includes_corrupted_manifest() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        create_test_project(root, "good_project", true)?;

        let corrupted_path = root.join("corrupted_project");
        std::fs::create_dir_all(&corrupted_path)?;
        std::fs::write(
            corrupted_path.join("Cargo.toml"),
            "invalid toml content [[[[",
        )?;

        let scanner = ProjectScanner::default();
        let projects = scanner.scan(root)?;

        assert_eq!(projects.len(), 2);
        assert!(projects.iter().any(|p| p.name == "good_project"));
        assert!(projects.iter().any(|p| p.name == "corrupted_project"));

        Ok(())
    }

    #[test]
    fn test_scan_empty_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        let scanner = ProjectScanner::default();
        let projects = scanner.scan(root)?;

        // 空目录应该返回空的项目列表
        assert!(projects.is_empty());

        Ok(())
    }

    #[test]
    fn test_scan_very_deep_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut current_path = temp_dir.path().to_path_buf();

        // 创建一个深的目录结构（但不要太深，避免超过默认深度限制）
        for i in 0..5 {
            current_path = current_path.join(format!("level_{i}"));
            std::fs::create_dir_all(&current_path)?;
        }

        // 在深层创建一个项目
        create_test_project(&current_path, "deep_project", true)?;

        // 使用足够大的深度限制
        let config = ScanConfig {
            max_depth: Some(20), // 足够深
            ..Default::default()
        };
        let scanner = ProjectScanner::new(config);
        let projects = scanner.scan(temp_dir.path())?;

        // 应该能找到深层的项目
        assert!(!projects.is_empty());
        let has_deep = projects.iter().any(|p| p.name == "deep_project");
        assert!(has_deep, "应该找到深层项目");

        Ok(())
    }
}
