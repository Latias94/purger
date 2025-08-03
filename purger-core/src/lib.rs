use serde::{Deserialize, Serialize};

pub mod cleaner;
pub mod filter;
pub mod project;
pub mod scanner;

pub use cleaner::{CleanPhase, CleanProgress, CleanStrategy, ProjectCleaner};
pub use filter::ProjectFilter;
pub use project::RustProject;
pub use scanner::ProjectScanner;

/// 清理结果统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanResult {
    pub cleaned_projects: usize,
    pub total_size_freed: u64,
    pub failed_projects: Vec<String>,
    pub duration_ms: u64,
}

impl Default for CleanResult {
    fn default() -> Self {
        Self::new()
    }
}

impl CleanResult {
    pub fn new() -> Self {
        Self {
            cleaned_projects: 0,
            total_size_freed: 0,
            failed_projects: Vec::new(),
            duration_ms: 0,
        }
    }

    pub fn add_success(&mut self, size_freed: u64) {
        self.cleaned_projects += 1;
        self.total_size_freed += size_freed;
    }

    pub fn add_failure(&mut self, project_path: String) {
        self.failed_projects.push(project_path);
    }

    pub fn format_size(&self) -> String {
        format_bytes(self.total_size_freed)
    }
}

/// 格式化字节大小为人类可读格式
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_clean_result() {
        let mut result = CleanResult::new();
        assert_eq!(result.cleaned_projects, 0);
        assert_eq!(result.total_size_freed, 0);

        result.add_success(1024);
        assert_eq!(result.cleaned_projects, 1);
        assert_eq!(result.total_size_freed, 1024);

        result.add_failure("test_project".to_string());
        assert_eq!(result.failed_projects.len(), 1);
    }
}
