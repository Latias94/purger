use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

/// Rust项目信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustProject {
    pub path: PathBuf,
    pub name: String,
    pub target_size: u64,
    pub last_modified: SystemTime,
    pub is_workspace: bool,
    pub has_target: bool,
}

impl RustProject {
    /// 从路径创建RustProject实例
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let cargo_toml_path = path.join("Cargo.toml");

        if !cargo_toml_path.exists() {
            anyhow::bail!("No Cargo.toml found at {:?}", path);
        }

        let name = Self::extract_project_name(&cargo_toml_path).unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string()
        });

        let is_workspace = Self::is_workspace_project(&cargo_toml_path)?;
        let target_path = path.join("target");
        let has_target = target_path.exists();

        let (target_size, last_modified) = if has_target {
            let modified = fs::metadata(&target_path)
                .context("Failed to get target directory metadata")?
                .modified()
                .context("Failed to get target directory modification time")?;
            // 延迟计算大小，只在需要时计算
            let size = Self::calculate_directory_size_fast(&target_path)?;
            (size, modified)
        } else {
            (0, SystemTime::UNIX_EPOCH)
        };

        Ok(RustProject {
            path,
            name,
            target_size,
            last_modified,
            is_workspace,
            has_target,
        })
    }

    /// 检查是否为workspace项目
    fn is_workspace_project(cargo_toml_path: &Path) -> Result<bool> {
        let content = fs::read_to_string(cargo_toml_path).context("Failed to read Cargo.toml")?;

        let parsed: toml::Value = toml::from_str(&content).context("Failed to parse Cargo.toml")?;

        Ok(parsed.get("workspace").is_some())
    }

    /// 从Cargo.toml提取项目名称
    fn extract_project_name(cargo_toml_path: &Path) -> Option<String> {
        let content = fs::read_to_string(cargo_toml_path).ok()?;
        let parsed: toml::Value = toml::from_str(&content).ok()?;

        parsed
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string())
    }

    /// 计算目录大小
    fn calculate_directory_size(dir: &Path) -> Result<u64> {
        let mut total_size = 0u64;

        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Ok(metadata) = entry.metadata() {
                    total_size += metadata.len();
                }
            }
        }

        Ok(total_size)
    }

    /// 快速计算目录大小（优化版本）
    fn calculate_directory_size_fast(dir: &Path) -> Result<u64> {
        use rayon::prelude::*;

        // 使用并行遍历来加速大目录的计算
        let entries: Vec<_> = WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect();

        let total_size: u64 = entries
            .par_iter()
            .filter_map(|entry| {
                entry.metadata().ok().map(|m| m.len())
            })
            .sum();

        Ok(total_size)
    }

    /// 获取格式化的大小字符串
    pub fn formatted_size(&self) -> String {
        crate::format_bytes(self.target_size)
    }

    /// 获取相对于给定基础路径的相对路径
    pub fn relative_path(&self, base: &Path) -> PathBuf {
        self.path
            .strip_prefix(base)
            .unwrap_or(&self.path)
            .to_path_buf()
    }

    /// 检查target目录是否存在
    pub fn target_exists(&self) -> bool {
        self.path.join("target").exists()
    }

    /// 获取target目录路径
    pub fn target_path(&self) -> PathBuf {
        self.path.join("target")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_is_workspace_project() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");

        // 测试普通项目
        fs::write(
            &cargo_toml,
            r#"
[package]
name = "test"
version = "0.1.0"
"#,
        )
        .unwrap();

        assert!(!RustProject::is_workspace_project(&cargo_toml).unwrap());

        // 测试workspace项目
        fs::write(
            &cargo_toml,
            r#"
[workspace]
members = ["crate1", "crate2"]
"#,
        )
        .unwrap();

        assert!(RustProject::is_workspace_project(&cargo_toml).unwrap());
    }

    #[test]
    fn test_extract_project_name() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");

        fs::write(
            &cargo_toml,
            r#"
[package]
name = "my-awesome-project"
version = "0.1.0"
"#,
        )
        .unwrap();

        let name = RustProject::extract_project_name(&cargo_toml);
        assert_eq!(name, Some("my-awesome-project".to_string()));
    }

    #[test]
    fn test_from_path_with_target() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let project_dir = temp_dir.path().join("test_project");
        std::fs::create_dir_all(&project_dir)?;

        // 创建Cargo.toml
        let cargo_toml = r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#;
        std::fs::write(project_dir.join("Cargo.toml"), cargo_toml)?;

        // 创建target目录和一些文件
        let target_dir = project_dir.join("target");
        std::fs::create_dir_all(&target_dir)?;
        std::fs::write(target_dir.join("test.txt"), "test content")?;

        let project = RustProject::from_path(&project_dir)?;
        assert_eq!(project.name, "test_project");
        assert!(project.has_target);
        assert!(project.target_size > 0);

        Ok(())
    }

    #[test]
    fn test_from_path_without_target() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let project_dir = temp_dir.path().join("test_project");
        std::fs::create_dir_all(&project_dir)?;

        // 创建Cargo.toml
        let cargo_toml = r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#;
        std::fs::write(project_dir.join("Cargo.toml"), cargo_toml)?;

        let project = RustProject::from_path(&project_dir)?;
        assert_eq!(project.name, "test_project");
        assert!(!project.has_target);
        assert_eq!(project.target_size, 0);

        Ok(())
    }

    #[test]
    fn test_from_path_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("invalid_project");
        std::fs::create_dir_all(&project_dir).unwrap();

        // 不创建Cargo.toml
        let result = RustProject::from_path(&project_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_formatted_size() {
        let project = RustProject {
            path: PathBuf::from("/test"),
            name: "test".to_string(),
            target_size: 1024,
            last_modified: SystemTime::now(),
            is_workspace: false,
            has_target: true,
        };

        let formatted = project.formatted_size();
        assert_eq!(formatted, "1.00 KB");
    }

    #[test]
    fn test_relative_path() {
        let project = RustProject {
            path: PathBuf::from("/home/user/projects/my_project"),
            name: "my_project".to_string(),
            target_size: 0,
            last_modified: SystemTime::now(),
            is_workspace: false,
            has_target: false,
        };

        let base = Path::new("/home/user/projects");
        let relative = project.relative_path(base);
        assert_eq!(relative, PathBuf::from("my_project"));
    }

    #[test]
    fn test_target_exists() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let project_dir = temp_dir.path().join("test_project");
        std::fs::create_dir_all(&project_dir)?;

        let project = RustProject {
            path: project_dir.clone(),
            name: "test".to_string(),
            target_size: 0,
            last_modified: SystemTime::now(),
            is_workspace: false,
            has_target: false,
        };

        // 最初target不存在
        assert!(!project.target_exists());

        // 创建target目录
        std::fs::create_dir_all(project_dir.join("target"))?;
        assert!(project.target_exists());

        Ok(())
    }

    #[test]
    fn test_target_path() {
        let project = RustProject {
            path: PathBuf::from("/test/project"),
            name: "test".to_string(),
            target_size: 0,
            last_modified: SystemTime::now(),
            is_workspace: false,
            has_target: false,
        };

        let target_path = project.target_path();
        assert_eq!(target_path, PathBuf::from("/test/project/target"));
    }
}
