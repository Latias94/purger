use crate::simple_i18n::Language;
use purger_core::CleanStrategy;
use serde::{Deserialize, Serialize};

/// 应用设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub recent_paths: Vec<String>,
    pub last_scan_path: String,
    pub max_depth: usize,
    pub target_only: bool,
    pub clean_strategy: CleanStrategy,
    pub auto_save_settings: bool,
    pub max_recent_paths: usize,
    pub language: Language,

    // 过滤选项
    pub keep_days: Option<u32>,
    pub keep_size_mb: Option<f64>, // 以MB为单位存储，便于UI显示
    pub ignore_paths: Vec<String>,

    // 可执行文件保留选项
    pub keep_executable: bool,
    pub executable_backup_dir: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            recent_paths: Vec::new(),
            last_scan_path: ".".to_string(),
            max_depth: 10,
            target_only: true,
            clean_strategy: CleanStrategy::CargoClean,
            auto_save_settings: true,
            max_recent_paths: 10,
            language: Language::default(),

            // 过滤选项默认值
            keep_days: None,
            keep_size_mb: None,
            ignore_paths: Vec::new(),

            // 可执行文件保留选项默认值
            keep_executable: false,
            executable_backup_dir: None,
        }
    }
}

impl AppSettings {
    /// 获取配置文件路径
    fn config_file_path() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|dir| dir.join("purger").join("settings.json"))
    }

    /// 从文件加载设置
    pub fn load_from_file() -> Self {
        if let Some(config_path) = Self::config_file_path() {
            if config_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&config_path) {
                    if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
                        tracing::info!("已加载配置文件: {:?}", config_path);
                        return settings;
                    }
                }
            }
        }

        tracing::info!("使用默认配置");
        Self::default()
    }

    /// 保存设置到文件
    pub fn save_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(config_path) = Self::config_file_path() {
            // 确保目录存在
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let content = serde_json::to_string_pretty(self)?;
            std::fs::write(&config_path, content)?;
            tracing::info!("已保存配置文件: {:?}", config_path);
        }
        Ok(())
    }

    /// 添加最近使用的路径
    pub fn add_recent_path(&mut self, path: String) {
        // 移除已存在的相同路径
        self.recent_paths.retain(|p| p != &path);

        // 添加到开头
        self.recent_paths.insert(0, path);

        // 限制数量
        if self.recent_paths.len() > self.max_recent_paths {
            self.recent_paths.truncate(self.max_recent_paths);
        }
    }

    /// 获取最近使用的路径
    #[allow(dead_code)]
    pub fn get_recent_paths(&self) -> &[String] {
        &self.recent_paths
    }

    /// 清除最近路径
    pub fn clear_recent_paths(&mut self) {
        self.recent_paths.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_settings_default() {
        let settings = AppSettings::default();

        assert!(settings.recent_paths.is_empty());
        assert_eq!(settings.last_scan_path, ".");
        assert_eq!(settings.max_depth, 10);
        assert!(settings.target_only);
        assert_eq!(settings.clean_strategy, CleanStrategy::CargoClean);
        assert!(settings.auto_save_settings);
        assert_eq!(settings.max_recent_paths, 10);
        assert_eq!(settings.language, Language::default());

        assert!(settings.keep_days.is_none());
        assert!(settings.keep_size_mb.is_none());
        assert!(settings.ignore_paths.is_empty());

        assert!(!settings.keep_executable);
        assert!(settings.executable_backup_dir.is_none());
    }

    #[test]
    fn test_add_recent_path() {
        let mut settings = AppSettings::default();

        // 添加第一个路径
        settings.add_recent_path("/path/1".to_string());
        assert_eq!(settings.recent_paths.len(), 1);
        assert_eq!(settings.recent_paths[0], "/path/1");

        // 添加第二个路径
        settings.add_recent_path("/path/2".to_string());
        assert_eq!(settings.recent_paths.len(), 2);
        assert_eq!(settings.recent_paths[0], "/path/2"); // 新路径在前面
        assert_eq!(settings.recent_paths[1], "/path/1");

        // 添加重复路径
        settings.add_recent_path("/path/1".to_string());
        assert_eq!(settings.recent_paths.len(), 2); // 数量不变
        assert_eq!(settings.recent_paths[0], "/path/1"); // 重复路径移到前面
        assert_eq!(settings.recent_paths[1], "/path/2");
    }

    #[test]
    fn test_add_recent_path_limit() {
        let mut settings = AppSettings {
            max_recent_paths: 3, // 设置最大数量为3
            ..Default::default()
        };

        // 添加4个路径
        settings.add_recent_path("/path/1".to_string());
        settings.add_recent_path("/path/2".to_string());
        settings.add_recent_path("/path/3".to_string());
        settings.add_recent_path("/path/4".to_string());

        // 应该只保留最新的3个
        assert_eq!(settings.recent_paths.len(), 3);
        assert_eq!(settings.recent_paths[0], "/path/4");
        assert_eq!(settings.recent_paths[1], "/path/3");
        assert_eq!(settings.recent_paths[2], "/path/2");
        // /path/1 应该被移除
    }

    #[test]
    fn test_get_recent_paths() {
        let mut settings = AppSettings::default();

        assert!(settings.get_recent_paths().is_empty());

        settings.add_recent_path("/path/1".to_string());
        settings.add_recent_path("/path/2".to_string());

        let paths = settings.get_recent_paths();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], "/path/2");
        assert_eq!(paths[1], "/path/1");
    }

    #[test]
    fn test_clear_recent_paths() {
        let mut settings = AppSettings::default();

        settings.add_recent_path("/path/1".to_string());
        settings.add_recent_path("/path/2".to_string());
        assert_eq!(settings.recent_paths.len(), 2);

        settings.clear_recent_paths();
        assert!(settings.recent_paths.is_empty());
    }

    #[test]
    fn test_serialization() {
        let settings = AppSettings::default();

        // 测试序列化
        let json = serde_json::to_string(&settings).unwrap();
        assert!(!json.is_empty());

        // 测试反序列化
        let deserialized: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.last_scan_path, settings.last_scan_path);
        assert_eq!(deserialized.max_depth, settings.max_depth);
        assert_eq!(deserialized.target_only, settings.target_only);
        assert_eq!(deserialized.clean_strategy, settings.clean_strategy);
    }

    #[test]
    fn test_config_file_path() {
        let path = AppSettings::config_file_path();

        // 在大多数系统上应该能获取到配置目录
        if let Some(config_path) = path {
            assert!(config_path.to_string_lossy().contains("purger"));
            assert!(config_path.to_string_lossy().contains("settings.json"));
        }
    }

    #[test]
    fn test_save_and_load_from_file() {
        use tempfile::TempDir;

        // 创建临时目录
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("purger").join("settings.json");

        // 创建测试设置
        let mut original_settings = AppSettings {
            last_scan_path: "/test/path".to_string(),
            max_depth: 5,
            ..Default::default()
        };
        original_settings.add_recent_path("/recent/1".to_string());
        original_settings.add_recent_path("/recent/2".to_string());

        // 模拟保存（这里我们直接写文件，因为save_to_file使用系统配置目录）
        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        let content = serde_json::to_string_pretty(&original_settings).unwrap();
        std::fs::write(&config_path, content).unwrap();

        // 模拟加载
        let loaded_content = std::fs::read_to_string(&config_path).unwrap();
        let loaded_settings: AppSettings = serde_json::from_str(&loaded_content).unwrap();

        // 验证加载的设置
        assert_eq!(loaded_settings.last_scan_path, "/test/path");
        assert_eq!(loaded_settings.max_depth, 5);
        assert_eq!(loaded_settings.recent_paths.len(), 2);
        assert_eq!(loaded_settings.recent_paths[0], "/recent/2");
        assert_eq!(loaded_settings.recent_paths[1], "/recent/1");
    }
}
