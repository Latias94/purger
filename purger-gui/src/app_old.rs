use std::path::PathBuf;
use std::sync::mpsc;
use eframe::egui;
use purger_core::{CleanPhase, cleaner::CleanConfig};

use crate::state::{AppSettings, AppState, AppMessage, AppData};
use crate::ui::{MenuBar, ScanPanel, ProjectList, ProgressBar, Dialogs};
use crate::handlers::{ScanHandler, CleanHandler};

/// 应用设置
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppSettings {
    recent_paths: Vec<String>,
    last_scan_path: String,
    max_depth: usize,
    target_only: bool,
    clean_strategy: CleanStrategy,
    auto_save_settings: bool,
    max_recent_paths: usize,
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
        }
    }
}

impl AppSettings {
    /// 获取配置文件路径
    fn config_file_path() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|dir| dir.join("purger").join("settings.json"))
    }

    /// 从文件加载设置
    fn load_from_file() -> Self {
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
    fn save_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
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
}

#[derive(Debug)]
enum AppMessage {
    ScanProgress(usize, usize), // (current, total)
    ScanComplete(Vec<RustProject>),
    ScanError(String),
    CleanProgress(usize, usize, u64), // (current, total, size_freed_so_far)
    CleanProjectStart(String), // project_name
    CleanProjectProgress(CleanProgress), // 详细的项目清理进度
    CleanProjectComplete(String, u64), // (project_name, size_freed)
    CleanComplete(CleanResult),
    CleanError(String),
}

#[derive(PartialEq)]
enum AppState {
    Idle,
    Scanning,
    Cleaning,
}

pub struct PurgerApp {
    // 设置
    settings: AppSettings,

    // UI状态
    scan_path: String,
    max_depth: String,
    show_settings: bool,
    show_about: bool,

    // 应用状态
    state: AppState,
    projects: Vec<RustProject>,
    selected_projects: Vec<bool>,

    // 进度状态
    scan_progress: Option<(usize, usize)>, // (current, total)
    clean_progress: Option<(usize, usize, u64)>, // (current, total, size_freed)
    current_cleaning_project: Option<String>, // 当前正在清理的项目名

    // 结果
    last_clean_result: Option<CleanResult>,
    error_message: Option<String>,

    // 通信和控制
    receiver: mpsc::Receiver<AppMessage>,
    sender: mpsc::Sender<AppMessage>,
    stop_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl PurgerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (sender, receiver) = mpsc::channel();

        // 从文件加载设置
        let settings = AppSettings::load_from_file();

        let scan_path = settings.last_scan_path.clone();
        let max_depth = settings.max_depth.to_string();

        Self {
            settings,
            scan_path,
            max_depth,
            show_settings: false,
            show_about: false,

            state: AppState::Idle,
            projects: Vec::new(),
            selected_projects: Vec::new(),

            scan_progress: None,
            clean_progress: None,
            current_cleaning_project: None,

            last_clean_result: None,
            error_message: None,

            receiver,
            sender,
            stop_requested: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// 保存设置
    fn save_settings(&mut self) {
        if self.settings.auto_save_settings {
            self.settings.last_scan_path = self.scan_path.clone();
            if let Ok(depth) = self.max_depth.parse::<usize>() {
                self.settings.max_depth = depth;
            }
            if let Err(e) = self.settings.save_to_file() {
                tracing::warn!("保存设置失败: {}", e);
            }
        }
    }

    /// 添加到最近路径
    fn add_recent_path(&mut self, path: String) {
        if !self.settings.recent_paths.contains(&path) {
            self.settings.recent_paths.insert(0, path);
            if self.settings.recent_paths.len() > self.settings.max_recent_paths {
                self.settings.recent_paths.truncate(self.settings.max_recent_paths);
            }
        }
    }

    /// 选择文件夹
    fn select_folder(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            let path_str = path.to_string_lossy().to_string();
            self.scan_path = path_str.clone();
            self.add_recent_path(path_str);
            // 保存设置
            self.save_settings();
        }
    }
    
    fn start_scan(&mut self) {
        if self.state != AppState::Idle {
            return;
        }

        self.state = AppState::Scanning;
        self.error_message = None;
        self.scan_progress = Some((0, 0));
        self.stop_requested.store(false, std::sync::atomic::Ordering::Relaxed);

        // 将扫描路径添加到最近使用
        self.add_recent_path(self.scan_path.clone());
        // 保存设置
        self.save_settings();

        let path = PathBuf::from(&self.scan_path);
        let max_depth = self.max_depth.parse().ok();
        let target_only = self.settings.target_only;
        let sender = self.sender.clone();
        let stop_flag = self.stop_requested.clone();

        thread::spawn(move || {
            let mut config = ScanConfig::default();
            config.max_depth = max_depth;

            let scanner = ProjectScanner::new(config);

            // 首先快速扫描获取项目数量
            let _ = sender.send(AppMessage::ScanProgress(0, 0));

            match scanner.scan(&path) {
                Ok(mut projects) => {
                    if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                        return;
                    }

                    let total = projects.len();
                    let _ = sender.send(AppMessage::ScanProgress(0, total));

                    // 模拟处理进度（实际中可以在项目解析时报告进度）
                    for (i, _) in projects.iter().enumerate() {
                        if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                            return;
                        }
                        let _ = sender.send(AppMessage::ScanProgress(i + 1, total));
                        // 小延迟以显示进度（实际使用中可以移除）
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }

                    if target_only {
                        projects = ProjectScanner::filter_with_target(projects);
                    }
                    projects = ProjectScanner::sort_by_size(projects);

                    if !stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                        let _ = sender.send(AppMessage::ScanComplete(projects));
                    }
                }
                Err(e) => {
                    if !stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                        let _ = sender.send(AppMessage::ScanError(e.to_string()));
                    }
                }
            }
        });
    }
    
    fn start_clean(&mut self) {
        if self.state != AppState::Idle || self.projects.is_empty() {
            return;
        }

        let selected_projects: Vec<RustProject> = self.projects
            .iter()
            .enumerate()
            .filter(|(i, _)| self.selected_projects.get(*i).copied().unwrap_or(false))
            .map(|(_, p)| p.clone())
            .collect();

        if selected_projects.is_empty() {
            self.error_message = Some("请选择要清理的项目".to_string());
            return;
        }

        self.state = AppState::Cleaning;
        self.error_message = None;
        self.clean_progress = Some((0, selected_projects.len(), 0));
        self.stop_requested.store(false, std::sync::atomic::Ordering::Relaxed);

        let strategy = self.settings.clean_strategy;
        let sender = self.sender.clone();
        let stop_flag = self.stop_requested.clone();

        thread::spawn(move || {
            let mut config = CleanConfig::default();
            config.strategy = strategy;

            let cleaner = ProjectCleaner::new(config);
            let total = selected_projects.len();
            let mut total_freed = 0u64;

            let _ = sender.send(AppMessage::CleanProgress(0, total, 0));

            for (i, project) in selected_projects.iter().enumerate() {
                if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }

                // 发送开始清理项目的消息
                let _ = sender.send(AppMessage::CleanProjectStart(project.name.clone()));

                // 使用带进度回调的清理方法
                let sender_clone = sender.clone();
                match cleaner.clean_project_with_progress(project, |progress| {
                    let _ = sender_clone.send(AppMessage::CleanProjectProgress(progress));
                }) {
                    Ok(size_freed) => {
                        total_freed += size_freed;
                        let _ = sender.send(AppMessage::CleanProjectComplete(project.name.clone(), size_freed));
                        let _ = sender.send(AppMessage::CleanProgress(i + 1, total, total_freed));
                    }
                    Err(e) => {
                        let _ = sender.send(AppMessage::CleanError(format!("清理项目 {} 失败: {}", project.name, e)));
                        return;
                    }
                }
            }

            if !stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                let mut result = purger_core::CleanResult::new();
                result.cleaned_projects = total;
                result.total_size_freed = total_freed;
                let _ = sender.send(AppMessage::CleanComplete(result));
            }
        });
    }
    
    fn handle_messages(&mut self) {
        while let Ok(message) = self.receiver.try_recv() {
            match message {
                AppMessage::ScanProgress(current, total) => {
                    self.scan_progress = Some((current, total));
                }
                AppMessage::ScanComplete(projects) => {
                    self.state = AppState::Idle;
                    self.scan_progress = None;
                    self.selected_projects = vec![false; projects.len()];
                    self.projects = projects;
                    self.stop_requested.store(false, std::sync::atomic::Ordering::Relaxed);
                }
                AppMessage::ScanError(error) => {
                    self.state = AppState::Idle;
                    self.scan_progress = None;
                    self.error_message = Some(format!("扫描失败: {}", error));
                    self.stop_requested.store(false, std::sync::atomic::Ordering::Relaxed);
                }
                AppMessage::CleanProgress(current, total, size_freed) => {
                    self.clean_progress = Some((current, total, size_freed));
                }
                AppMessage::CleanProjectStart(project_name) => {
                    self.current_cleaning_project = Some(project_name);
                }
                AppMessage::CleanProjectProgress(progress) => {
                    // 更新当前清理项目的详细进度
                    self.current_cleaning_project = Some(format!(
                        "{} - {} ({}/{})",
                        progress.project_name,
                        match progress.phase {
                            CleanPhase::Starting => "开始",
                            CleanPhase::Analyzing => "分析",
                            CleanPhase::Cleaning => "清理中",
                            CleanPhase::Finalizing => "完成",
                            CleanPhase::Complete => "完成",
                        },
                        progress.files_processed,
                        progress.total_files.unwrap_or(0)
                    ));
                }
                AppMessage::CleanProjectComplete(project_name, _size_freed) => {
                    // 项目清理完成，可以在这里添加更详细的日志
                    tracing::info!("项目 {} 清理完成", project_name);
                }
                AppMessage::CleanComplete(result) => {
                    self.state = AppState::Idle;
                    self.clean_progress = None;
                    self.current_cleaning_project = None;
                    self.last_clean_result = Some(result);
                    self.stop_requested.store(false, std::sync::atomic::Ordering::Relaxed);
                    // 重新扫描以更新项目状态
                    self.start_scan();
                }
                AppMessage::CleanError(error) => {
                    self.state = AppState::Idle;
                    self.clean_progress = None;
                    self.current_cleaning_project = None;
                    self.error_message = Some(format!("清理失败: {}", error));
                    self.stop_requested.store(false, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
    }

    /// 停止当前操作
    fn stop_operation(&mut self) {
        self.stop_requested.store(true, std::sync::atomic::Ordering::Relaxed);
        self.state = AppState::Idle;
        self.scan_progress = None;
        self.clean_progress = None;
        self.current_cleaning_project = None;
    }
}

impl eframe::App for PurgerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_messages();

        // 菜单栏
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("文件", |ui| {
                    if ui.button("选择文件夹...").clicked() {
                        self.select_folder();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("退出").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("设置", |ui| {
                    if ui.button("首选项...").clicked() {
                        self.show_settings = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button("帮助", |ui| {
                    if ui.button("关于...").clicked() {
                        self.show_about = true;
                        ui.close_menu();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // 扫描配置区域
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("扫描路径:");
                        ui.add(egui::TextEdit::singleline(&mut self.scan_path).desired_width(400.0));

                        if ui.button("📁").clicked() {
                            self.select_folder();
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("最大深度:");
                        ui.add(egui::TextEdit::singleline(&mut self.max_depth).desired_width(60.0));

                        ui.separator();
                        ui.checkbox(&mut self.settings.target_only, "仅显示有target的项目");

                        // 最近使用的路径
                        if !self.settings.recent_paths.is_empty() {
                            ui.separator();
                            ui.label("最近使用:");
                            egui::ComboBox::from_id_source("recent_paths")
                                .selected_text("选择...")
                                .width(150.0)
                                .show_ui(ui, |ui| {
                                    for path in &self.settings.recent_paths.clone() {
                                        if ui.selectable_label(false, path).clicked() {
                                            self.scan_path = path.clone();
                                        }
                                    }
                                });
                        }
                    });
                });
            });

            // 操作按钮和状态
            ui.horizontal(|ui| {
                let can_scan = self.state == AppState::Idle;
                let can_stop = self.state != AppState::Idle;

                if ui.add_enabled(can_scan, egui::Button::new("扫描")).clicked() {
                    self.start_scan();
                }

                if ui.add_enabled(can_stop, egui::Button::new("停止")).clicked() {
                    self.stop_operation();
                }

                ui.separator();

                // 状态和进度
                match self.state {
                    AppState::Idle => {
                        ui.label("就绪");
                    }
                    AppState::Scanning => {
                        ui.label("扫描中...");
                        if let Some((current, total)) = self.scan_progress {
                            if total > 0 {
                                let progress = current as f32 / total as f32;
                                ui.add(egui::ProgressBar::new(progress).text(format!("{current}/{total}")));
                            } else {
                                ui.spinner();
                            }
                        }
                    }
                    AppState::Cleaning => {
                        ui.label("清理中...");
                        if let Some((current, total, size_freed)) = self.clean_progress {
                            let progress = current as f32 / total as f32;
                            ui.add(egui::ProgressBar::new(progress).text(format!("{current}/{total}")));
                            ui.label(format!("已释放: {}", purger_core::format_bytes(size_freed)));

                            // 显示当前正在清理的项目
                            if let Some(project_name) = &self.current_cleaning_project {
                                ui.label(format!("正在清理: {project_name}"));
                            }
                        }
                    }
                }
            });

            ui.separator();
            
            // 错误消息
            if let Some(error) = &self.error_message {
                ui.colored_label(egui::Color32::RED, error);
                ui.separator();
            }
            
            // 清理结果
            if let Some(result) = &self.last_clean_result {
                ui.colored_label(
                    egui::Color32::GREEN,
                    format!("上次清理: 成功{}个项目，释放{}", 
                           result.cleaned_projects, 
                           result.format_size())
                );
                ui.separator();
            }
            
            // 项目列表
            if !self.projects.is_empty() {
                // 计算选中项目的可清理大小
                let (selected_count, total_cleanable_size): (usize, u64) = self.projects
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| self.selected_projects.get(*i).copied().unwrap_or(false))
                    .map(|(_, p)| (1, if p.has_target { p.target_size } else { 0 }))
                    .fold((0, 0), |(count, size), (c, s)| (count + c, size + s));

                // 清理策略
                ui.horizontal(|ui| {
                    ui.label("清理策略:");
                    ui.radio_value(&mut self.settings.clean_strategy, CleanStrategy::CargoClean, "Cargo Clean");
                    ui.radio_value(&mut self.settings.clean_strategy, CleanStrategy::DirectDelete, "直接删除");
                });

                // 统计信息和操作按钮
                ui.horizontal(|ui| {
                    if selected_count > 0 {
                        ui.label(format!("已选中: {} 个项目", selected_count));
                    }

                    if total_cleanable_size > 0 {
                        ui.label(format!("可清理: {}", purger_core::format_bytes(total_cleanable_size)));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let can_clean = self.state == AppState::Idle && selected_count > 0;
                        if ui.add_enabled(can_clean, egui::Button::new("清理选中项目")).clicked() {
                            self.start_clean();
                        }
                    });
                });

                ui.horizontal(|ui| {
                    if ui.button("全选").clicked() {
                        self.selected_projects.fill(true);
                    }
                    if ui.button("全不选").clicked() {
                        self.selected_projects.fill(false);
                    }
                    if ui.button("反选").clicked() {
                        for selected in &mut self.selected_projects {
                            *selected = !*selected;
                        }
                    }
                });

                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.style_mut().spacing.item_spacing.y = 2.0; // 减少行间距

                    for (i, project) in self.projects.iter().enumerate() {
                        // 确保selected_projects有足够的元素
                        while self.selected_projects.len() <= i {
                            self.selected_projects.push(false);
                        }

                        // 使用固定高度的区域来避免布局问题
                        ui.allocate_ui_with_layout(
                            egui::Vec2::new(ui.available_width(), 40.0),
                            egui::Layout::left_to_right(egui::Align::Min),
                            |ui| {
                                ui.checkbox(&mut self.selected_projects[i], "");

                                // 项目信息区域，限制宽度
                                ui.allocate_ui_with_layout(
                                    egui::Vec2::new(ui.available_width() - 20.0, 40.0), // 留出滚动条空间
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        // 第一行：项目名称和大小
                                        ui.horizontal(|ui| {
                                            ui.label(&project.name);
                                            if project.has_target {
                                                ui.colored_label(egui::Color32::DARK_GREEN, format!("({})", project.formatted_size()));
                                            } else {
                                                ui.colored_label(egui::Color32::GRAY, "(无target)");
                                            }
                                        });

                                        // 第二行：项目路径
                                        ui.horizontal(|ui| {
                                            let path_text = project.path.to_string_lossy();
                                            ui.colored_label(egui::Color32::GRAY, path_text);
                                        });
                                    }
                                );
                            }
                        );

                        ui.separator();
                    }
                });
            } else {
                ui.label("点击扫描按钮开始查找Rust项目");
            }
        });

        // 设置对话框
        if self.show_settings {
            egui::Window::new("设置")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("最大最近路径数:");
                        ui.add(egui::DragValue::new(&mut self.settings.max_recent_paths).range(1..=20));
                    });

                    ui.checkbox(&mut self.settings.auto_save_settings, "自动保存设置");

                    ui.horizontal(|ui| {
                        if ui.button("清除最近路径").clicked() {
                            self.settings.recent_paths.clear();
                        }

                        if ui.button("重置为默认").clicked() {
                            self.settings = AppSettings::default();
                        }
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("确定").clicked() {
                            self.show_settings = false;
                        }
                        if ui.button("取消").clicked() {
                            self.show_settings = false;
                        }
                    });
                });
        }

        // 关于对话框
        if self.show_about {
            egui::Window::new("关于")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Rust Project Purger");
                        ui.label("版本 0.1.0");
                        ui.separator();
                        ui.label("一个用于清理Rust项目构建目录的工具");
                        ui.label("支持批量扫描和选择性清理");
                        ui.separator();
                        ui.label("使用egui构建 • 开源软件");

                        if ui.button("确定").clicked() {
                            self.show_about = false;
                        }
                    });
                });
        }

        // 定期刷新UI
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.save_settings();
    }
}
