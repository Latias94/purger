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
        "scan.max_depth_label" => "最大深度:",
        "scan.strategy_label" => "清理策略:",
        "scan.recent_paths_label" => "最近路径:",
        "scan.recent_paths_placeholder" => "选择最近路径...",
        "scan.start_button" => "开始扫描",
        "scan.scanning_status" => "正在扫描...",
        "scan.strategy_cargo_clean" => "Cargo Clean (推荐)",
        "scan.strategy_direct_delete" => "直接删除",
        "projects.empty_message" => "点击扫描按钮开始查找Rust项目",
        "projects.found_message" => "找到 %{count} 个Rust项目",
        "projects.selected_message" => "已选中: %{count} 个项目",
        "projects.cleanable_size" => "可清理: %{size}",
        "projects.clean_button" => "清理选中项目",
        "projects.select_all" => "全选",
        "projects.select_none" => "全不选",
        "projects.invert_selection" => "反选",
        "progress.scan_label" => "扫描进度:",
        "progress.clean_label" => "清理进度:",
        "progress.current_project" => "当前项目:",
        "progress.freed_size" => "已释放:",
        "progress.last_result" => "上次清理结果:",
        "dialog.settings_title" => "设置",
        "dialog.about_title" => "关于",
        "dialog.max_recent_paths" => "最大最近路径数:",
        "dialog.auto_save_settings" => "自动保存设置",
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
        "scan.max_depth_label" => "Max Depth:",
        "scan.strategy_label" => "Clean Strategy:",
        "scan.recent_paths_label" => "Recent Paths:",
        "scan.recent_paths_placeholder" => "Select recent path...",
        "scan.start_button" => "Start Scan",
        "scan.scanning_status" => "Scanning...",
        "scan.strategy_cargo_clean" => "Cargo Clean (Recommended)",
        "scan.strategy_direct_delete" => "Direct Delete",
        "projects.empty_message" => "Click scan button to start finding Rust projects",
        "projects.found_message" => "Found %{count} Rust projects",
        "projects.selected_message" => "Selected: %{count} projects",
        "projects.cleanable_size" => "Cleanable: %{size}",
        "projects.clean_button" => "Clean Selected Projects",
        "projects.select_all" => "Select All",
        "projects.select_none" => "Select None",
        "projects.invert_selection" => "Invert Selection",
        "progress.scan_label" => "Scan Progress:",
        "progress.clean_label" => "Clean Progress:",
        "progress.current_project" => "Current Project:",
        "progress.freed_size" => "Freed:",
        "progress.last_result" => "Last Clean Result:",
        "dialog.settings_title" => "Settings",
        "dialog.about_title" => "About",
        "dialog.max_recent_paths" => "Max Recent Paths:",
        "dialog.auto_save_settings" => "Auto Save Settings",
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
