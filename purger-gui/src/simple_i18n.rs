use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// 支持的语言
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "zh-CN")]
    Chinese,
    #[serde(rename = "en")]
    English,
}

impl Language {
    /// 获取所有支持的语言
    pub fn all() -> Vec<Language> {
        vec![Language::Chinese, Language::English]
    }

    /// 获取语言代码
    #[allow(dead_code)]
    pub fn code(&self) -> &'static str {
        match self {
            Language::Chinese => "zh-CN",
            Language::English => "en",
        }
    }

    /// 获取语言显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::Chinese => "中文",
            Language::English => "English",
        }
    }

    /// 从语言代码创建语言
    #[allow(dead_code)]
    pub fn from_code(code: &str) -> Option<Language> {
        match code {
            "zh-CN" => Some(Language::Chinese),
            "en" => Some(Language::English),
            _ => None,
        }
    }
}

impl Default for Language {
    fn default() -> Self {
        detect_system_language()
    }
}

/// 检测系统语言
pub fn detect_system_language() -> Language {
    // 尝试获取系统语言环境
    if let Some(locale) = sys_locale::get_locale() {
        tracing::info!("检测到系统语言环境: {}", locale);

        // 检查是否为中文环境
        if locale.starts_with("zh") {
            tracing::info!("使用中文界面");
            return Language::Chinese;
        }

        // 检查是否为英文环境
        if locale.starts_with("en") {
            tracing::info!("使用英文界面");
            return Language::English;
        }

        tracing::info!("未识别的语言环境 '{}', 使用英文作为默认语言", locale);
    } else {
        tracing::warn!("无法检测系统语言环境，使用英文作为默认语言");
    }

    // 默认使用英文
    Language::English
}

// 全局语言状态
static CURRENT_LANGUAGE: RwLock<Option<Language>> = RwLock::new(None);

/// 设置当前语言
pub fn set_language(language: Language) {
    if let Ok(mut lang) = CURRENT_LANGUAGE.write() {
        tracing::info!("切换语言到: {:?}", language);
        *lang = Some(language);
    }
}

/// 获取当前语言
pub fn current_language() -> Language {
    if let Ok(lang) = CURRENT_LANGUAGE.read() {
        if let Some(language) = *lang {
            return language;
        }
    }

    // 如果没有设置过语言，使用系统检测的语言
    let detected = detect_system_language();
    set_language(detected);
    detected
}

/// 翻译宏
#[macro_export]
macro_rules! tr {
    ($key:expr) => {
        $crate::simple_i18n::translate($key)
    };
    ($key:expr, $($name:ident = $value:expr),*) => {{
        let mut result = $crate::simple_i18n::translate($key);
        $(
            result = result.replace(&format!("%{{{}}}", stringify!($name)), &$value.to_string());
        )*
        result
    }};
}

/// 翻译函数
pub fn translate(key: &str) -> String {
    let lang = current_language();
    match lang {
        Language::Chinese => translate_chinese(key),
        Language::English => translate_english(key),
    }
    .unwrap_or_else(|| key.to_string())
}

fn translate_chinese(key: &str) -> Option<String> {
    let text = match key {
        "app.title" => "Rust Project Purger",
        "menu.file" => "文件",
        "menu.settings" => "设置",
        "menu.help" => "帮助",
        "menu.select_folder" => "选择文件夹...",
        "menu.exit" => "退出",
        "menu.preferences" => "首选项...",
        "menu.about" => "关于...",
        "scan.path_label" => "扫描路径:",
        "scan.path_hint" => "例如: . 或 /path/to/projects",
        "scan.max_depth_label" => "最大深度:",
        "scan.strategy_label" => "清理策略:",
        "scan.recent_paths_label" => "最近路径:",
        "scan.recent_paths_placeholder" => "选择最近路径...",
        "scan.start_button" => "开始扫描",
        "scan.browse_button" => "选择...",
        "scan.stop_button" => "停止",
        "scan.scanning_status" => "正在扫描...",
        "scan.sizing_status" => "正在计算大小...",
        "scan.strategy_cargo_clean" => "Cargo Clean (推荐)",
        "scan.strategy_direct_delete" => "直接删除",
        "filters.title" => "筛选",
        "filters.search_label" => "搜索",
        "filters.search_placeholder" => "按名称或路径搜索...",
        "filters.clear_search" => "清空",
        "filters.sort_label" => "排序:",
        "filters.sort.size_desc" => "按大小(大→小)",
        "filters.sort.size_asc" => "按大小(小→大)",
        "filters.sort.modified_desc" => "按最近编译(新→旧)",
        "filters.sort.modified_asc" => "按最近编译(旧→新)",
        "filters.sort.name_asc" => "按名称(A→Z)",
        "filters.sort.name_desc" => "按名称(Z→A)",
        "filters.sort.path_asc" => "按路径",
        "filters.sort.path_desc" => "按路径(倒序)",
        "filters.selected_only" => "只看已选",
        "filters.target_only" => "只显示可清理(有 target)",
        "filters.workspace_only" => "只看 workspace 根",
        "filters.scan_filters" => "扫描过滤(需重新扫描)",
        "filters.scan_filters_hint" => "这些选项会影响扫描结果；修改后需要重新扫描。",
        "filters.advanced" => "高级选项",
        "filters.advanced_hint" => {
            "部分选项会影响扫描结果（修改后建议重新扫描）；大小筛选需要先计算大小。"
        }
        "filters.keep_days_label" => "保留最近编译(天)",
        "filters.keep_days_hint" => "留空=不过滤",
        "filters.keep_size_label" => "保留小项目(MB)",
        "filters.keep_size_hint" => "留空=不过滤",
        "filters.keep_executable" => "保留可执行文件",
        "filters.backup_dir" => "备份目录:",
        "filters.backup_dir_hint" => "留空=不备份",
        "filters.ignore_paths" => "忽略路径",
        "filters.ignore_add" => "添加",
        "filters.ignore_remove" => "删除",
        "projects.empty_message" => "点击扫描按钮开始查找Rust项目",
        "projects.no_match" => "没有符合当前筛选条件的项目",
        "projects.waiting_sizes" => "正在计算大小以应用筛选...",
        "projects.found_message" => "找到 %{count} 个Rust项目",
        "projects.showing_message" => "显示 %{visible}/%{total}",
        "projects.selected_message" => "已选中: %{count} 个项目",
        "projects.cleanable_size" => "可清理: %{size}",
        "projects.clean_button" => "清理选中项目",
        "projects.select_all" => "全选",
        "projects.select_none" => "全不选",
        "projects.invert_selection" => "反选",
        "projects.column_name" => "项目",
        "projects.column_size" => "大小",
        "projects.column_modified" => "最近编译",
        "projects.column_path" => "路径",
        "projects.column_tags" => "标签",
        "projects.tag_workspace" => "workspace",
        "projects.no_target" => "无 target",
        "details.title" => "详情",
        "details.empty" => "点击列表中的项目查看详情",
        "details.not_found" => "项目不存在或已被移除",
        "details.path_label" => "路径",
        "details.copy_path" => "复制路径",
        "details.open_project" => "打开项目",
        "details.open_target" => "打开 target",
        "details.open_failed" => "打开失败",
        "details.size_label" => "target 大小:",
        "details.modified_label" => "最近编译:",
        "details.selected" => "已选中",
        "details.select_only" => "仅选中此项",
        "details.time_unknown" => "未知",
        "details.time_just_now" => "刚刚",
        "details.time_minutes" => "%{n} 分钟前",
        "details.time_hours" => "%{n} 小时前",
        "details.time_days" => "%{n} 天前",
        "actions.no_selection" => "未选择任何项目",
        "actions.size_calculating" => "大小计算中…",
        "actions.select_cleanable" => "全选(可清理)",
        "clean.confirm_title" => "确认清理",
        "clean.confirm_message" => "将清理 %{count} 个项目，预计释放 %{size}",
        "clean.confirm_strategy" => "策略: %{strategy}",
        "clean.confirm_button" => "开始清理",
        "progress.scan_label" => "扫描进度:",
        "progress.scan_found" => "已发现 %{count} 个Cargo.toml",
        "progress.size_label" => "大小计算:",
        "progress.clean_label" => "清理进度:",
        "progress.current_project" => "当前项目:",
        "progress.freed_size" => "已释放:",
        "progress.last_result" => "上次清理结果:",
        "progress.cleaned_projects" => "成功清理: %{count}",
        "progress.freed_space" => "释放空间: %{size}",
        "progress.duration" => "耗时: %{ms} ms",
        "progress.failed_projects" => "失败: %{count}",
        "progress.failed_so_far" => "已失败: %{count}",
        "progress.failed_details" => "失败详情 (%{count})",
        "progress.copy_failed" => "复制失败详情",
        "dialog.settings_title" => "设置",
        "dialog.about_title" => "关于",
        "dialog.max_recent_paths" => "最大最近路径数:",
        "dialog.auto_save_settings" => "自动保存设置",
        "dialog.clean_timeout" => "清理超时(秒, 0=禁用):",
        "dialog.clear_recent_paths" => "清除最近路径",
        "dialog.reset_defaults" => "重置为默认",
        "dialog.ok" => "确定",
        "dialog.cancel" => "取消",
        "about.version" => "版本 0.1.0",
        "about.description1" => "一个用于清理Rust项目构建目录的工具",
        "about.description2" => "支持批量扫描和选择性清理",
        "about.footer" => "使用egui构建 • 开源软件",
        "strategy.cargo_clean" => "Cargo Clean",
        "strategy.direct_delete" => "直接删除",
        "language.label" => "语言:",
        _ => return None,
    };
    Some(text.to_string())
}

fn translate_english(key: &str) -> Option<String> {
    let text = match key {
        "app.title" => "Rust Project Purger",
        "menu.file" => "File",
        "menu.settings" => "Settings",
        "menu.help" => "Help",
        "menu.select_folder" => "Select Folder...",
        "menu.exit" => "Exit",
        "menu.preferences" => "Preferences...",
        "menu.about" => "About...",
        "scan.path_label" => "Scan Path:",
        "scan.path_hint" => "e.g. . or /path/to/projects",
        "scan.max_depth_label" => "Max Depth:",
        "scan.strategy_label" => "Clean Strategy:",
        "scan.recent_paths_label" => "Recent Paths:",
        "scan.recent_paths_placeholder" => "Select recent path...",
        "scan.start_button" => "Start Scan",
        "scan.browse_button" => "Browse...",
        "scan.stop_button" => "Stop",
        "scan.scanning_status" => "Scanning...",
        "scan.sizing_status" => "Calculating sizes...",
        "scan.strategy_cargo_clean" => "Cargo Clean (Recommended)",
        "scan.strategy_direct_delete" => "Direct Delete",
        "filters.title" => "Filters",
        "filters.search_label" => "Search",
        "filters.search_placeholder" => "Search by name or path...",
        "filters.clear_search" => "Clear",
        "filters.sort_label" => "Sort:",
        "filters.sort.size_desc" => "Size (desc)",
        "filters.sort.size_asc" => "Size (asc)",
        "filters.sort.modified_desc" => "Last build (new→old)",
        "filters.sort.modified_asc" => "Last build (old→new)",
        "filters.sort.name_asc" => "Name (A→Z)",
        "filters.sort.name_desc" => "Name (Z→A)",
        "filters.sort.path_asc" => "Path",
        "filters.sort.path_desc" => "Path (desc)",
        "filters.selected_only" => "Selected only",
        "filters.target_only" => "Only cleanable (has target)",
        "filters.workspace_only" => "Workspace roots only",
        "filters.scan_filters" => "Scan Filters (re-scan needed)",
        "filters.scan_filters_hint" => "These options affect scan results; re-scan after changes.",
        "filters.advanced" => "Advanced",
        "filters.advanced_hint" => {
            "Some options affect scan results (re-scan recommended); size filter needs size calculation."
        }
        "filters.keep_days_label" => "Keep recent (days)",
        "filters.keep_days_hint" => "Empty = no filter",
        "filters.keep_size_label" => "Keep small (MB)",
        "filters.keep_size_hint" => "Empty = no filter",
        "filters.keep_executable" => "Keep executables",
        "filters.backup_dir" => "Backup dir:",
        "filters.backup_dir_hint" => "Empty = no backup",
        "filters.ignore_paths" => "Ignore paths",
        "filters.ignore_add" => "Add",
        "filters.ignore_remove" => "Remove",
        "projects.empty_message" => "Click scan button to start finding Rust projects",
        "projects.no_match" => "No projects match the current filters",
        "projects.waiting_sizes" => "Calculating sizes to apply filters...",
        "projects.found_message" => "Found %{count} Rust projects",
        "projects.showing_message" => "Showing %{visible}/%{total}",
        "projects.selected_message" => "Selected: %{count} projects",
        "projects.cleanable_size" => "Cleanable: %{size}",
        "projects.clean_button" => "Clean Selected Projects",
        "projects.select_all" => "Select All",
        "projects.select_none" => "Select None",
        "projects.invert_selection" => "Invert Selection",
        "projects.column_name" => "Project",
        "projects.column_size" => "Size",
        "projects.column_modified" => "Last build",
        "projects.column_path" => "Path",
        "projects.column_tags" => "Tags",
        "projects.tag_workspace" => "workspace",
        "projects.no_target" => "no target",
        "details.title" => "Details",
        "details.empty" => "Select a project to see details",
        "details.not_found" => "Project not found",
        "details.path_label" => "Path",
        "details.copy_path" => "Copy Path",
        "details.open_project" => "Open Project",
        "details.open_target" => "Open target",
        "details.open_failed" => "Open failed",
        "details.size_label" => "target size:",
        "details.modified_label" => "Last build:",
        "details.selected" => "Selected",
        "details.select_only" => "Select only",
        "details.time_unknown" => "unknown",
        "details.time_just_now" => "just now",
        "details.time_minutes" => "%{n} min ago",
        "details.time_hours" => "%{n} h ago",
        "details.time_days" => "%{n} d ago",
        "actions.no_selection" => "No selection",
        "actions.size_calculating" => "Calculating sizes…",
        "actions.select_cleanable" => "Select cleanable",
        "clean.confirm_title" => "Confirm Clean",
        "clean.confirm_message" => "Clean %{count} projects, estimate %{size} freed",
        "clean.confirm_strategy" => "Strategy: %{strategy}",
        "clean.confirm_button" => "Start Cleaning",
        "progress.scan_label" => "Scan Progress:",
        "progress.scan_found" => "Found %{count} Cargo.toml",
        "progress.size_label" => "Size calculation:",
        "progress.clean_label" => "Clean Progress:",
        "progress.current_project" => "Current Project:",
        "progress.freed_size" => "Freed:",
        "progress.last_result" => "Last Clean Result:",
        "progress.cleaned_projects" => "Cleaned: %{count}",
        "progress.freed_space" => "Freed: %{size}",
        "progress.duration" => "Duration: %{ms} ms",
        "progress.failed_projects" => "Failed: %{count}",
        "progress.failed_so_far" => "Failed: %{count}",
        "progress.failed_details" => "Failure details (%{count})",
        "progress.copy_failed" => "Copy failures",
        "dialog.settings_title" => "Settings",
        "dialog.about_title" => "About",
        "dialog.max_recent_paths" => "Max Recent Paths:",
        "dialog.auto_save_settings" => "Auto Save Settings",
        "dialog.clean_timeout" => "Clean timeout (sec, 0=disabled):",
        "dialog.clear_recent_paths" => "Clear Recent Paths",
        "dialog.reset_defaults" => "Reset to Defaults",
        "dialog.ok" => "OK",
        "dialog.cancel" => "Cancel",
        "about.version" => "Version 0.1.0",
        "about.description1" => "A tool for cleaning Rust project build directories",
        "about.description2" => "Supports batch scanning and selective cleaning",
        "about.footer" => "Built with egui • Open Source Software",
        "strategy.cargo_clean" => "Cargo Clean",
        "strategy.direct_delete" => "Direct Delete",
        "language.label" => "Language:",
        _ => return None,
    };
    Some(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_display_name() {
        assert_eq!(Language::Chinese.display_name(), "中文");
        assert_eq!(Language::English.display_name(), "English");
    }

    #[test]
    fn test_translation() {
        set_language(Language::English);
        assert_eq!(translate("menu.file"), "File");

        set_language(Language::Chinese);
        assert_eq!(translate("menu.file"), "文件");
    }

    #[test]
    fn test_translation_with_params() {
        set_language(Language::English);
        let result = translate("projects.found_message").replace("%{count}", "5");
        assert_eq!(result, "Found 5 Rust projects");

        set_language(Language::Chinese);
        let result = translate("projects.found_message").replace("%{count}", "5");
        assert_eq!(result, "找到 5 个Rust项目");
    }

    #[test]
    fn test_unknown_key() {
        assert_eq!(translate("unknown.key"), "unknown.key");
    }

    #[test]
    fn test_language_switching() {
        // 明确设置语言并验证
        set_language(Language::Chinese);
        assert_eq!(current_language(), Language::Chinese);

        set_language(Language::English);
        assert_eq!(current_language(), Language::English);

        // 再次切换回中文
        set_language(Language::Chinese);
        assert_eq!(current_language(), Language::Chinese);
    }

    #[test]
    fn test_system_language_detection() {
        let detected = detect_system_language();
        // 应该返回有效的语言
        assert!(matches!(detected, Language::Chinese | Language::English));
    }

    #[test]
    fn test_language_all() {
        let languages = Language::all();
        assert_eq!(languages.len(), 2);
        assert!(languages.contains(&Language::Chinese));
        assert!(languages.contains(&Language::English));
    }

    #[test]
    fn test_language_code() {
        assert_eq!(Language::Chinese.code(), "zh-CN");
        assert_eq!(Language::English.code(), "en");
    }

    #[test]
    fn test_language_from_code() {
        assert_eq!(Language::from_code("zh-CN"), Some(Language::Chinese));
        assert_eq!(Language::from_code("en"), Some(Language::English));
        assert_eq!(Language::from_code("fr"), None);
        assert_eq!(Language::from_code(""), None);
        assert_eq!(Language::from_code("invalid"), None);
    }

    #[test]
    fn test_language_serialization() {
        // 测试序列化
        let chinese_json = serde_json::to_string(&Language::Chinese).unwrap();
        let english_json = serde_json::to_string(&Language::English).unwrap();

        assert_eq!(chinese_json, "\"zh-CN\"");
        assert_eq!(english_json, "\"en\"");

        // 测试反序列化
        let chinese: Language = serde_json::from_str("\"zh-CN\"").unwrap();
        let english: Language = serde_json::from_str("\"en\"").unwrap();

        assert_eq!(chinese, Language::Chinese);
        assert_eq!(english, Language::English);
    }

    #[test]
    fn test_language_equality() {
        assert_eq!(Language::Chinese, Language::Chinese);
        assert_eq!(Language::English, Language::English);
        assert_ne!(Language::Chinese, Language::English);
    }

    #[test]
    fn test_language_clone() {
        let lang = Language::Chinese;
        let cloned = lang;
        assert_eq!(lang, cloned);
    }

    #[test]
    fn test_language_debug() {
        let debug_str = format!("{:?}", Language::Chinese);
        assert!(debug_str.contains("Chinese"));
    }
}
