use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    pub fn code(&self) -> &'static str {
        match self {
            Language::Chinese => "zh-CN",
            Language::English => "en",
        }
    }

    /// 获取语言显示名称
    pub fn display_name(&self) -> String {
        match self {
            Language::Chinese => "中文".to_string(),
            Language::English => "English".to_string(),
        }
    }

    /// 从语言代码创建语言
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
        // 根据系统语言设置默认语言
        let locale = sys_locale::get_locale().unwrap_or_else(|| "en".to_string());
        if locale.starts_with("zh") {
            Language::Chinese
        } else {
            Language::English
        }
    }
}

// 全局语言状态
static CURRENT_LANGUAGE: RwLock<Language> = RwLock::new(Language::English);

/// 设置当前语言
pub fn set_language(language: Language) {
    if let Ok(mut lang) = CURRENT_LANGUAGE.write() {
        *lang = language;
    }
}

/// 获取当前语言
pub fn current_language() -> Language {
    CURRENT_LANGUAGE.read().unwrap_or_else(|_| Language::default().into()).clone()
}

/// 简单的翻译函数
pub fn t(key: &str) -> String {
    let lang = current_language();
    get_translation(key, lang)
}

/// 带参数的翻译函数
pub fn t_with_args(key: &str, args: &HashMap<&str, String>) -> String {
    let mut result = t(key);
    for (k, v) in args {
        result = result.replace(&format!("%{{{}}}", k), v);
    }
    result
}

/// 获取翻译文本
fn get_translation(key: &str, language: Language) -> String {
    match language {
        Language::Chinese => get_chinese_translation(key),
        Language::English => get_english_translation(key),
    }.unwrap_or_else(|| key.to_string())
}

/// 中文翻译
fn get_chinese_translation(key: &str) -> Option<String> {
    let translations = [
        // 应用程序
        ("app.title", "Rust Project Purger"),

        // 菜单栏
        ("menu.file", "文件"),
        ("menu.settings", "设置"),
        ("menu.help", "帮助"),
        ("menu.select_folder", "选择文件夹..."),
        ("menu.exit", "退出"),
        ("menu.preferences", "首选项..."),
        ("menu.about", "关于..."),

        // 扫描面板
        ("scan.path_label", "扫描路径:"),
        ("scan.max_depth_label", "最大深度:"),
        ("scan.strategy_label", "清理策略:"),
        ("scan.recent_paths_label", "最近路径:"),
        ("scan.recent_paths_placeholder", "选择最近路径..."),
        ("scan.start_button", "开始扫描"),
        ("scan.scanning_status", "正在扫描..."),
        ("scan.strategy_cargo_clean", "Cargo Clean (推荐)"),
        ("scan.strategy_direct_delete", "直接删除"),

        // 项目列表
        ("projects.empty_message", "点击扫描按钮开始查找Rust项目"),
        ("projects.found_message", "找到 %{count} 个Rust项目"),
        ("projects.selected_message", "已选中: %{count} 个项目"),
        ("projects.cleanable_size", "可清理: %{size}"),
        ("projects.clean_button", "清理选中项目"),
        ("projects.select_all", "全选"),
        ("projects.select_none", "全不选"),
        ("projects.invert_selection", "反选"),

        // 进度显示
        ("progress.scan_label", "扫描进度:"),
        ("progress.clean_label", "清理进度:"),
        ("progress.current_project", "当前项目:"),
        ("progress.freed_size", "已释放:"),
        ("progress.last_result", "上次清理结果:"),
        ("progress.cleaned_projects", "成功清理: %{count} 个项目"),
        ("progress.freed_space", "释放空间: %{size}"),
        ("progress.duration", "耗时: %{ms}ms"),
        ("progress.failed_projects", "失败项目: %{count} 个"),

        // 对话框
        ("dialog.settings_title", "设置"),
        ("dialog.about_title", "关于"),
        ("dialog.max_recent_paths", "最大最近路径数:"),
        ("dialog.auto_save_settings", "自动保存设置"),
        ("dialog.clear_recent_paths", "清除最近路径"),
        ("dialog.reset_defaults", "重置为默认"),
        ("dialog.ok", "确定"),
        ("dialog.cancel", "取消"),

        // 关于对话框
        ("about.version", "版本 0.1.0"),
        ("about.description1", "一个用于清理Rust项目构建目录的工具"),
        ("about.description2", "支持批量扫描和选择性清理"),
        ("about.footer", "使用egui构建 • 开源软件"),

        // 清理策略
        ("strategy.cargo_clean", "Cargo Clean"),
        ("strategy.direct_delete", "直接删除"),

        // 语言设置
        ("language.label", "语言:"),
        ("language.chinese", "中文"),
        ("language.english", "English"),
    ];

    translations.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.to_string())
}

/// 英文翻译
fn get_english_translation(key: &str) -> Option<String> {
    let translations = [
        // Application
        ("app.title", "Rust Project Purger"),

        // Menu bar
        ("menu.file", "File"),
        ("menu.settings", "Settings"),
        ("menu.help", "Help"),
        ("menu.select_folder", "Select Folder..."),
        ("menu.exit", "Exit"),
        ("menu.preferences", "Preferences..."),
        ("menu.about", "About..."),

        // Scan panel
        ("scan.path_label", "Scan Path:"),
        ("scan.max_depth_label", "Max Depth:"),
        ("scan.strategy_label", "Clean Strategy:"),
        ("scan.recent_paths_label", "Recent Paths:"),
        ("scan.recent_paths_placeholder", "Select recent path..."),
        ("scan.start_button", "Start Scan"),
        ("scan.scanning_status", "Scanning..."),
        ("scan.strategy_cargo_clean", "Cargo Clean (Recommended)"),
        ("scan.strategy_direct_delete", "Direct Delete"),

        // Project list
        ("projects.empty_message", "Click scan button to start finding Rust projects"),
        ("projects.found_message", "Found %{count} Rust projects"),
        ("projects.selected_message", "Selected: %{count} projects"),
        ("projects.cleanable_size", "Cleanable: %{size}"),
        ("projects.clean_button", "Clean Selected Projects"),
        ("projects.select_all", "Select All"),
        ("projects.select_none", "Select None"),
        ("projects.invert_selection", "Invert Selection"),

        // Progress display
        ("progress.scan_label", "Scan Progress:"),
        ("progress.clean_label", "Clean Progress:"),
        ("progress.current_project", "Current Project:"),
        ("progress.freed_size", "Freed:"),
        ("progress.last_result", "Last Clean Result:"),
        ("progress.cleaned_projects", "Successfully cleaned: %{count} projects"),
        ("progress.freed_space", "Freed space: %{size}"),
        ("progress.duration", "Duration: %{ms}ms"),
        ("progress.failed_projects", "Failed projects: %{count}"),

        // Dialogs
        ("dialog.settings_title", "Settings"),
        ("dialog.about_title", "About"),
        ("dialog.max_recent_paths", "Max Recent Paths:"),
        ("dialog.auto_save_settings", "Auto Save Settings"),
        ("dialog.clear_recent_paths", "Clear Recent Paths"),
        ("dialog.reset_defaults", "Reset to Defaults"),
        ("dialog.ok", "OK"),
        ("dialog.cancel", "Cancel"),

        // About dialog
        ("about.version", "Version 0.1.0"),
        ("about.description1", "A tool for cleaning Rust project build directories"),
        ("about.description2", "Supports batch scanning and selective cleaning"),
        ("about.footer", "Built with egui • Open Source Software"),

        // Clean strategy
        ("strategy.cargo_clean", "Cargo Clean"),
        ("strategy.direct_delete", "Direct Delete"),

        // Language settings
        ("language.label", "Language:"),
        ("language.chinese", "中文"),
        ("language.english", "English"),
    ];

    translations.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_code() {
        assert_eq!(Language::Chinese.code(), "zh-CN");
        assert_eq!(Language::English.code(), "en");
    }

    #[test]
    fn test_language_from_code() {
        assert_eq!(Language::from_code("zh-CN"), Some(Language::Chinese));
        assert_eq!(Language::from_code("en"), Some(Language::English));
        assert_eq!(Language::from_code("invalid"), None);
    }

    #[test]
    fn test_translation() {
        set_language(Language::English);
        assert_eq!(crate::t!("menu.file"), "File");

        set_language(Language::Chinese);
        assert_eq!(crate::t!("menu.file"), "文件");
    }
}
