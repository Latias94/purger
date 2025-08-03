use eframe::egui;
use purger_core::{CleanPhase, cleaner::CleanConfig};
use std::path::PathBuf;
use std::sync::mpsc;

use crate::handlers::{CleanHandler, ScanHandler};
use crate::simple_i18n::{Language, detect_system_language, set_language};
use crate::state::{AppData, AppMessage, AppSettings, AppState};
use crate::ui::{Dialogs, MenuBar, ProgressBar, ProjectList, ScanPanel};

/// 主应用结构
pub struct PurgerApp {
    // 设置
    settings: AppSettings,

    // UI状态
    scan_path: String,
    max_depth: String,
    show_settings: bool,
    show_about: bool,

    // 应用状态和数据
    state: AppState,
    data: AppData,

    // 通信和控制
    receiver: mpsc::Receiver<AppMessage>,
    sender: mpsc::Sender<AppMessage>,
    stop_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl PurgerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (sender, receiver) = mpsc::channel();

        // 从文件加载设置
        let mut settings = AppSettings::load_from_file();

        // 如果设置中没有保存语言偏好，使用系统检测的语言
        if settings.language == Language::default() {
            let detected_lang = detect_system_language();
            tracing::info!("首次运行，检测到系统语言: {:?}", detected_lang);
            settings.language = detected_lang;
        }

        // 设置当前语言
        set_language(settings.language);

        let scan_path = settings.last_scan_path.clone();
        let max_depth = settings.max_depth.to_string();

        Self {
            settings,
            scan_path,
            max_depth,
            show_settings: false,
            show_about: false,

            state: AppState::Idle,
            data: AppData::new(),

            receiver,
            sender,
            stop_requested: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// 获取设置的引用
    #[allow(dead_code)]
    pub fn settings(&self) -> &AppSettings {
        &self.settings
    }

    /// 处理消息
    fn handle_messages(&mut self) {
        while let Ok(message) = self.receiver.try_recv() {
            match message {
                AppMessage::ScanProgress(current, total) => {
                    self.data.scan_progress = Some((current, total));
                }
                AppMessage::ScanComplete(projects) => {
                    self.state = AppState::Idle;
                    self.data.scan_progress = None;
                    self.data.set_projects(projects);
                    self.stop_requested
                        .store(false, std::sync::atomic::Ordering::Relaxed);

                    // 保存扫描路径到设置
                    self.settings.last_scan_path = self.scan_path.clone();
                    self.settings.add_recent_path(self.scan_path.clone());
                    self.save_settings();
                }
                AppMessage::ScanError(error) => {
                    self.state = AppState::Idle;
                    self.data.scan_progress = None;
                    self.data.error_message = Some(format!("扫描失败: {error}"));
                    self.stop_requested
                        .store(false, std::sync::atomic::Ordering::Relaxed);
                }
                AppMessage::CleanProgress(current, total, size_freed) => {
                    self.data.clean_progress = Some((current, total, size_freed));
                }
                AppMessage::CleanProjectStart(project_name) => {
                    self.data.current_cleaning_project = Some(project_name);
                }
                AppMessage::CleanProjectProgress(progress) => {
                    // 更新当前清理项目的详细进度
                    self.data.current_cleaning_project = Some(format!(
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
                    self.data.clean_progress = None;
                    self.data.current_cleaning_project = None;
                    self.data.last_clean_result = Some(result);
                    self.stop_requested
                        .store(false, std::sync::atomic::Ordering::Relaxed);
                    // 重新扫描以更新项目状态
                    self.start_scan();
                }
                AppMessage::CleanError(error) => {
                    self.state = AppState::Idle;
                    self.data.clean_progress = None;
                    self.data.current_cleaning_project = None;
                    self.data.error_message = Some(format!("清理失败: {error}"));
                    self.stop_requested
                        .store(false, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
    }

    /// 开始扫描
    fn start_scan(&mut self) {
        let path = PathBuf::from(&self.scan_path);
        let max_depth = self.max_depth.parse().ok();

        self.state = AppState::Scanning;
        self.data.error_message = None;
        self.data.scan_progress = Some((0, 0));
        self.stop_requested
            .store(false, std::sync::atomic::Ordering::Relaxed);

        ScanHandler::start_scan(
            path,
            max_depth,
            self.settings.clone(),
            self.sender.clone(),
            self.stop_requested.clone(),
        );
    }

    /// 开始清理
    fn start_clean(&mut self) {
        let selected_projects: Vec<_> = self
            .data
            .get_selected_projects()
            .into_iter()
            .cloned()
            .collect();

        if selected_projects.is_empty() {
            return;
        }

        self.state = AppState::Cleaning;
        self.data.error_message = None;
        self.data.clean_progress = Some((0, selected_projects.len(), 0));
        self.stop_requested
            .store(false, std::sync::atomic::Ordering::Relaxed);

        let config = CleanConfig {
            strategy: self.settings.clean_strategy,
            keep_executable: self.settings.keep_executable,
            executable_backup_dir: self
                .settings
                .executable_backup_dir
                .as_ref()
                .map(std::path::PathBuf::from),
            ..Default::default()
        };

        CleanHandler::start_clean(
            selected_projects,
            config,
            self.sender.clone(),
            self.stop_requested.clone(),
        );
    }

    /// 选择文件夹
    fn select_folder(&mut self) {
        if let Some(path) = ScanHandler::select_folder() {
            self.scan_path = path;
        }
    }

    /// 保存设置
    fn save_settings(&self) {
        if self.settings.auto_save_settings {
            if let Err(e) = self.settings.save_to_file() {
                tracing::error!("保存设置失败: {}", e);
            }
        }
    }
}

impl eframe::App for PurgerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_messages();

        // 处理UI事件
        let mut on_select_folder = false;
        let mut on_start_scan = false;
        let mut on_start_clean = false;

        // 菜单栏
        MenuBar::show(
            ctx,
            &mut self.show_settings,
            &mut self.show_about,
            &mut on_select_folder,
        );

        egui::CentralPanel::default().show(ctx, |ui| {
            // 扫描配置区域
            ScanPanel::show(
                ui,
                &mut self.scan_path,
                &mut self.max_depth,
                &mut self.settings,
                &self.state,
                &mut on_select_folder,
                &mut on_start_scan,
            );

            ui.separator();

            // 进度显示
            ProgressBar::show_all_progress(ui, &self.state, &self.data);

            ui.separator();

            // 项目列表
            ProjectList::show(ui, &mut self.data, &self.state, &mut on_start_clean);
        });

        // 对话框
        Dialogs::show_settings(ctx, &mut self.show_settings, &mut self.settings);
        Dialogs::show_about(ctx, &mut self.show_about);

        // 处理事件
        if on_select_folder {
            self.select_folder();
        }
        if on_start_scan {
            self.start_scan();
        }
        if on_start_clean {
            self.start_clean();
        }

        // 定期刷新UI
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.save_settings();
    }
}
