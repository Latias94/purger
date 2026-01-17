use eframe::egui;
use purger_core::{CleanPhase, cleaner::CleanConfig};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::SystemTime;

use crate::handlers::{CleanHandler, ScanHandler};
use crate::simple_i18n::{Language, detect_system_language, set_language};
use crate::state::{AppData, AppMessage, AppSettings, AppState};
use crate::tr;
use crate::ui::{
    ActionBar, Dialogs, FiltersPanel, MenuBar, ProgressBar, ProjectDetails, ProjectList,
    ProjectSort, ScanPanel,
};

/// Main application
pub struct PurgerApp {
    // 设置
    settings: AppSettings,

    // UI状态
    scan_path: String,
    show_settings: bool,
    show_about: bool,
    show_clean_confirm: bool,

    // 列表视图状态
    search_query: String,
    sort: ProjectSort,
    show_selected_only: bool,

    // 应用状态和数据
    state: AppState,
    data: AppData,

    // 通信和控制
    receiver: mpsc::Receiver<AppMessage>,
    sender: mpsc::Sender<AppMessage>,
    stop_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl PurgerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (sender, receiver) = mpsc::channel();

        apply_compact_style(&cc.egui_ctx);

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

        Self {
            settings,
            scan_path,
            show_settings: false,
            show_about: false,
            show_clean_confirm: false,

            search_query: String::new(),
            sort: ProjectSort::SizeDesc,
            show_selected_only: false,

            state: AppState::Idle,
            data: AppData::new(),

            receiver,
            sender,
            stop_requested: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Handle incoming messages
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
                    let total = progress
                        .total_files
                        .map(|t| t.to_string())
                        .unwrap_or_else(|| "?".to_string());
                    self.data.current_cleaning_project = Some(format!(
                        "{} - {} ({}/{total})",
                        progress.project_name,
                        match progress.phase {
                            CleanPhase::Starting => "开始",
                            CleanPhase::Analyzing => "分析",
                            CleanPhase::Cleaning => "清理中",
                            CleanPhase::Finalizing => "完成",
                            CleanPhase::Complete => "完成",
                        },
                        progress.files_processed,
                    ));
                }
                AppMessage::CleanProjectComplete(project_name, _size_freed) => {
                    tracing::info!("项目 {} 清理完成", project_name);
                }
                AppMessage::CleanProjectError(project_name, error) => {
                    self.data
                        .clean_errors
                        .push((project_name.clone(), error.clone()));
                    self.data.error_message =
                        Some(format!("项目 {project_name} 清理失败: {error}"));
                }
                AppMessage::CleanComplete(result) => {
                    self.state = AppState::Idle;
                    self.data.clean_progress = None;
                    self.data.current_cleaning_project = None;
                    self.data.last_clean_result = Some(result);
                    self.data.error_message = None;
                    self.stop_requested
                        .store(false, std::sync::atomic::Ordering::Relaxed);
                    self.start_scan();
                }
            }
        }
    }

    /// Start scanning
    fn start_scan(&mut self) {
        let path = PathBuf::from(&self.scan_path);
        let max_depth = if self.settings.max_depth == 0 {
            None
        } else {
            Some(self.settings.max_depth)
        };

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

    /// Start cleaning
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
        self.data.clean_errors.clear();
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

    /// Stop the current operation
    fn stop_operation(&mut self) {
        self.stop_requested
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.state = AppState::Idle;
        self.data.scan_progress = None;
        self.data.clean_progress = None;
        self.data.current_cleaning_project = None;
    }

    /// Select a folder
    fn select_folder(&mut self) {
        if let Some(path) = ScanHandler::select_folder() {
            self.scan_path = path;
        }
    }

    /// Save settings
    fn save_settings(&self) {
        if self.settings.auto_save_settings {
            if let Err(e) = self.settings.save_to_file() {
                tracing::error!("保存设置失败: {}", e);
            }
        }
    }

    fn visible_project_indices(&self) -> Vec<usize> {
        let query = self.search_query.trim().to_lowercase();
        let mut indices: Vec<usize> = (0..self.data.projects.len()).collect();

        indices.retain(|&i| {
            let project = &self.data.projects[i];

            if self.settings.target_only && !project.has_target {
                return false;
            }
            if self.show_selected_only && !self.data.is_selected(project) {
                return false;
            }
            if query.is_empty() {
                return true;
            }

            project.name.to_lowercase().contains(&query)
                || project
                    .path
                    .display()
                    .to_string()
                    .to_lowercase()
                    .contains(&query)
        });

        indices.sort_by(|&a, &b| {
            let pa = &self.data.projects[a];
            let pb = &self.data.projects[b];
            match self.sort {
                ProjectSort::SizeDesc => pb.target_size.cmp(&pa.target_size),
                ProjectSort::ModifiedDesc => {
                    system_time_key(pb.last_modified).cmp(&system_time_key(pa.last_modified))
                }
                ProjectSort::NameAsc => pa.name.cmp(&pb.name),
                ProjectSort::PathAsc => pa.path.cmp(&pb.path),
            }
        });

        indices
    }
}

impl eframe::App for PurgerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_messages();

        let mut on_select_folder = false;
        let mut on_start_scan = false;
        let mut on_stop = false;
        let mut on_request_clean = false;
        let mut on_confirm_clean = false;

        // 菜单栏
        MenuBar::show(
            ctx,
            &mut self.show_settings,
            &mut self.show_about,
            &mut on_select_folder,
        );

        // 顶部扫描工具栏
        egui::TopBottomPanel::top("scan_toolbar").show(ctx, |ui| {
            ScanPanel::show(
                ui,
                &mut self.scan_path,
                &mut self.settings,
                &self.state,
                &mut on_select_folder,
                &mut on_start_scan,
                &mut on_stop,
            );
        });

        // 左侧筛选
        egui::SidePanel::left("filters_panel")
            .default_width(240.0)
            .resizable(true)
            .show(ctx, |ui| {
                FiltersPanel::show(
                    ui,
                    &mut self.settings,
                    &mut self.search_query,
                    &mut self.sort,
                    &mut self.show_selected_only,
                );
            });

        // 右侧详情
        egui::SidePanel::right("details_panel")
            .default_width(280.0)
            .resizable(true)
            .show(ctx, |ui| {
                ProjectDetails::show(ui, &mut self.data);
            });

        // 底部操作栏 + 进度
        egui::TopBottomPanel::bottom("actions_panel").show(ctx, |ui| {
            ProgressBar::show_all_progress(ui, &self.state, &self.data);
            ui.separator();
            ActionBar::show(ui, &mut self.data, &self.state, &mut on_request_clean);
        });

        // 中间主列表
        let visible = self.visible_project_indices();
        egui::CentralPanel::default().show(ctx, |ui| {
            ProjectList::show(ui, &mut self.data, &self.state, &visible);
        });

        // 对话框
        Dialogs::show_settings(ctx, &mut self.show_settings, &mut self.settings);
        Dialogs::show_about(ctx, &mut self.show_about);

        if self.show_clean_confirm {
            let selected_count = self.data.get_selected_count();
            let total_selected_size = self.data.get_total_cleanable_size();
            let strategy_text = match self.settings.clean_strategy {
                purger_core::CleanStrategy::CargoClean => tr!("strategy.cargo_clean"),
                purger_core::CleanStrategy::DirectDelete => tr!("strategy.direct_delete"),
            };

            egui::Window::new(tr!("clean.confirm_title"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(tr!(
                        "clean.confirm_message",
                        count = selected_count,
                        size = purger_core::format_bytes(total_selected_size)
                    ));
                    ui.label(tr!("clean.confirm_strategy", strategy = strategy_text));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(tr!("dialog.cancel")).clicked() {
                            self.show_clean_confirm = false;
                        }
                        let can_confirm = selected_count > 0 && self.state == AppState::Idle;
                        if ui
                            .add_enabled(
                                can_confirm,
                                egui::Button::new(tr!("clean.confirm_button")),
                            )
                            .clicked()
                        {
                            on_confirm_clean = true;
                            self.show_clean_confirm = false;
                        }
                    });
                });
        }

        // 处理事件
        if on_select_folder {
            self.select_folder();
        }
        if on_start_scan {
            self.start_scan();
        }
        if on_stop {
            self.stop_operation();
        }
        if on_request_clean {
            self.show_clean_confirm = true;
        }
        if on_confirm_clean {
            self.start_clean();
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.save_settings();
    }
}

fn system_time_key(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn apply_compact_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.spacing.item_spacing = egui::vec2(6.0, 2.0);
    style.spacing.button_padding = egui::vec2(6.0, 2.0);
    style.spacing.window_margin = egui::Margin::same(8);
    style.spacing.menu_margin = egui::Margin::same(6);
    style.spacing.indent = 14.0;
    style.spacing.interact_size = egui::vec2(32.0, 18.0);

    style.text_styles = [
        (egui::TextStyle::Heading, egui::FontId::proportional(18.0)),
        (egui::TextStyle::Body, egui::FontId::proportional(13.0)),
        (egui::TextStyle::Monospace, egui::FontId::monospace(12.0)),
        (egui::TextStyle::Button, egui::FontId::proportional(13.0)),
        (egui::TextStyle::Small, egui::FontId::proportional(11.0)),
    ]
    .into();

    ctx.set_style(style);
}
