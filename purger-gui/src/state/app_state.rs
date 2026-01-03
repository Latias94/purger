use purger_core::{CleanProgress, CleanResult, RustProject};
use std::collections::HashSet;
use std::path::PathBuf;

/// Application runtime state
#[derive(PartialEq, Debug, Clone)]
pub enum AppState {
    Idle,
    Scanning,
    Cleaning,
}

/// Application message types
#[derive(Debug)]
pub enum AppMessage {
    ScanProgress(usize, usize), // (current, total)
    ScanComplete(Vec<RustProject>),
    ScanError(String),
    CleanProgress(usize, usize, u64), // (current, total, size_freed_so_far)
    CleanProjectStart(String),        // project_name
    CleanProjectProgress(CleanProgress), // 详细的项目清理进度
    CleanProjectComplete(String, u64), // (project_name, size_freed)
    CleanProjectError(String, String), // (project_name, error)
    CleanComplete(CleanResult),
}

/// Application data model
#[derive(Default)]
pub struct AppData {
    // 项目数据
    pub projects: Vec<RustProject>,
    pub selected_projects: HashSet<PathBuf>,
    pub focused_project: Option<PathBuf>,

    // 进度状态
    pub scan_progress: Option<(usize, usize)>, // (current, total)
    pub clean_progress: Option<(usize, usize, u64)>, // (current, total, size_freed)
    pub current_cleaning_project: Option<String>, // 当前正在清理的项目名
    pub clean_errors: Vec<(String, String)>,

    // 结果
    pub last_clean_result: Option<CleanResult>,
    pub error_message: Option<String>,
}

impl AppData {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the project list
    pub fn set_projects(&mut self, projects: Vec<RustProject>) {
        self.projects = projects;
        self.selected_projects.clear();
        self.focused_project = None;
    }

    /// Get selected projects
    pub fn get_selected_projects(&self) -> Vec<&RustProject> {
        self.projects
            .iter()
            .filter(|project| project.has_target && self.selected_projects.contains(&project.path))
            .collect()
    }

    /// Get selected count
    pub fn get_selected_count(&self) -> usize {
        self.selected_projects.len()
    }

    /// Get total cleanable size
    pub fn get_total_cleanable_size(&self) -> u64 {
        self.projects
            .iter()
            .filter_map(|project| {
                if project.has_target && self.selected_projects.contains(&project.path) {
                    Some(project.target_size)
                } else {
                    None
                }
            })
            .sum()
    }

    /// Check if a project is selected
    pub fn is_selected(&self, project: &RustProject) -> bool {
        if !project.has_target {
            return false;
        }
        self.selected_projects.contains(&project.path)
    }

    /// Set selection state for a project
    pub fn set_selected(&mut self, project: &RustProject, selected: bool) {
        if !project.has_target {
            self.selected_projects.remove(&project.path);
            return;
        }
        if selected {
            self.selected_projects.insert(project.path.clone());
        } else {
            self.selected_projects.remove(&project.path);
        }
    }

    /// Select only the given project
    pub fn select_only(&mut self, project: &RustProject) {
        self.selected_projects.clear();
        if project.has_target {
            self.selected_projects.insert(project.path.clone());
        }
        self.focused_project = Some(project.path.clone());
    }

    /// Select all cleanable projects (those with `target`)
    pub fn select_all_cleanable(&mut self) {
        for project in &self.projects {
            if project.has_target {
                self.selected_projects.insert(project.path.clone());
            }
        }
    }

    /// Select all
    pub fn select_all(&mut self) {
        self.select_all_cleanable();
    }

    /// Select none
    pub fn select_none(&mut self) {
        self.selected_projects.clear();
    }

    /// Invert selection
    pub fn invert_selection(&mut self) {
        let mut next = HashSet::new();
        for project in &self.projects {
            if project.has_target && !self.selected_projects.contains(&project.path) {
                next.insert(project.path.clone());
            }
        }
        self.selected_projects = next;
    }

    /// Clear the last error message
    #[allow(dead_code)]
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Reset progress state
    #[allow(dead_code)]
    pub fn reset_progress(&mut self) {
        self.scan_progress = None;
        self.clean_progress = None;
        self.current_cleaning_project = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn create_test_project(name: &str, target_size: u64, has_target: bool) -> RustProject {
        RustProject {
            path: std::path::PathBuf::from(format!("/test/{name}")),
            name: name.to_string(),
            target_size,
            last_modified: SystemTime::now(),
            is_workspace: false,
            has_target,
        }
    }

    #[test]
    fn test_app_data_new() {
        let data = AppData::new();
        assert!(data.projects.is_empty());
        assert!(data.selected_projects.is_empty());
        assert!(data.scan_progress.is_none());
        assert!(data.clean_progress.is_none());
        assert!(data.current_cleaning_project.is_none());
        assert!(data.clean_errors.is_empty());
        assert!(data.last_clean_result.is_none());
        assert!(data.error_message.is_none());
    }

    #[test]
    fn test_set_projects() {
        let mut data = AppData::new();
        let projects = vec![
            create_test_project("project1", 1000, true),
            create_test_project("project2", 2000, true),
            create_test_project("project3", 0, false),
        ];

        data.set_projects(projects.clone());

        assert_eq!(data.projects.len(), 3);
        assert_eq!(data.selected_projects.len(), 0);
        assert_eq!(data.projects[0].name, "project1");
        assert_eq!(data.projects[1].name, "project2");
        assert_eq!(data.projects[2].name, "project3");

        // 默认都不选中
        assert_eq!(data.get_selected_count(), 0);
    }

    #[test]
    fn test_get_selected_projects() {
        let mut data = AppData::new();
        let projects = vec![
            create_test_project("project1", 1000, true),
            create_test_project("project2", 2000, true),
            create_test_project("project3", 3000, false),
        ];

        data.set_projects(projects);

        // 选中第一个和第二个项目（可清理）
        let p0 = data.projects[0].clone();
        let p1 = data.projects[1].clone();
        data.set_selected(&p0, true);
        data.set_selected(&p1, true);

        let selected = data.get_selected_projects();
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].name, "project1");
        assert_eq!(selected[1].name, "project2");
    }

    #[test]
    fn test_get_selected_count() {
        let mut data = AppData::new();
        let projects = vec![
            create_test_project("project1", 1000, true),
            create_test_project("project2", 2000, true),
            create_test_project("project3", 3000, false),
        ];

        data.set_projects(projects);

        assert_eq!(data.get_selected_count(), 0);

        let p0 = data.projects[0].clone();
        data.set_selected(&p0, true);
        assert_eq!(data.get_selected_count(), 1);

        // 无 target 的项目不会被选中
        let p2 = data.projects[2].clone();
        data.set_selected(&p2, true);
        assert_eq!(data.get_selected_count(), 1);

        let p1 = data.projects[1].clone();
        data.set_selected(&p1, true);
        assert_eq!(data.get_selected_count(), 2);
    }

    #[test]
    fn test_get_total_cleanable_size() {
        let mut data = AppData::new();
        let projects = vec![
            create_test_project("project1", 1000, true), // 有target，可清理
            create_test_project("project2", 2000, true), // 有target，可清理
            create_test_project("project3", 3000, false), // 无target，不可清理
        ];

        data.set_projects(projects);

        // 没有选中任何项目
        assert_eq!(data.get_total_cleanable_size(), 0);

        // 选中第一个项目（有target）
        let p0 = data.projects[0].clone();
        data.set_selected(&p0, true);
        assert_eq!(data.get_total_cleanable_size(), 1000);

        // 选中第三个项目（无target）
        let p2 = data.projects[2].clone();
        data.set_selected(&p2, true);
        assert_eq!(data.get_selected_count(), 1);
        assert_eq!(data.get_total_cleanable_size(), 1000); // 仍然是1000，因为project3没有target

        // 选中第二个项目（有target）
        let p1 = data.projects[1].clone();
        data.set_selected(&p1, true);
        assert_eq!(data.get_total_cleanable_size(), 3000); // 1000 + 2000
    }

    #[test]
    fn test_select_all() {
        let mut data = AppData::new();
        let projects = vec![
            create_test_project("project1", 1000, true),
            create_test_project("project2", 2000, true),
            create_test_project("project3", 3000, false),
        ];

        data.set_projects(projects);
        data.select_all();

        // 仅会选中可清理项目
        assert_eq!(data.get_selected_count(), 2);
    }

    #[test]
    fn test_select_none() {
        let mut data = AppData::new();
        let projects = vec![
            create_test_project("project1", 1000, true),
            create_test_project("project2", 2000, true),
        ];

        data.set_projects(projects);
        data.select_all();
        assert_eq!(data.get_selected_count(), 2);

        data.select_none();
        assert_eq!(data.get_selected_count(), 0);
    }

    #[test]
    fn test_invert_selection() {
        let mut data = AppData::new();
        let projects = vec![
            create_test_project("project1", 1000, true),
            create_test_project("project2", 2000, true),
            create_test_project("project3", 3000, false),
        ];

        data.set_projects(projects);

        // 选中第一个项目
        let p0 = data.projects[0].clone();
        data.set_selected(&p0, true);
        assert_eq!(data.get_selected_count(), 1);

        // 反选
        data.invert_selection();
        assert_eq!(data.get_selected_count(), 1);
        assert!(!data.is_selected(&data.projects[0])); // 原来选中的变成未选中
        assert!(data.is_selected(&data.projects[1])); // 原来未选中的变成选中
        assert!(!data.is_selected(&data.projects[2])); // 无 target 的项目不会被选中
    }

    #[test]
    fn test_clear_error() {
        let mut data = AppData::new();
        data.error_message = Some("Test error".to_string());

        assert!(data.error_message.is_some());
        data.clear_error();
        assert!(data.error_message.is_none());
    }

    #[test]
    fn test_reset_progress() {
        let mut data = AppData::new();
        data.scan_progress = Some((5, 10));
        data.clean_progress = Some((3, 8, 1024));
        data.current_cleaning_project = Some("test_project".to_string());

        data.reset_progress();

        assert!(data.scan_progress.is_none());
        assert!(data.clean_progress.is_none());
        assert!(data.current_cleaning_project.is_none());
    }

    #[test]
    fn test_app_state_enum() {
        let idle = AppState::Idle;
        let scanning = AppState::Scanning;
        let cleaning = AppState::Cleaning;

        assert_eq!(idle, AppState::Idle);
        assert_ne!(idle, scanning);
        assert_ne!(scanning, cleaning);

        // 测试Clone
        let idle_clone = idle.clone();
        assert_eq!(idle, idle_clone);
    }

    #[test]
    fn test_app_message_debug() {
        let msg1 = AppMessage::ScanProgress(5, 10);
        let _msg2 = AppMessage::ScanError("Test error".to_string());
        let _msg3 = AppMessage::CleanProgress(3, 8, 1024);

        // 测试Debug trait
        let debug_str = format!("{msg1:?}");
        assert!(debug_str.contains("ScanProgress"));
        assert!(debug_str.contains("5"));
        assert!(debug_str.contains("10"));
    }
}
