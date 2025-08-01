use crate::state::AppMessage;
use purger_core::{cleaner::CleanConfig, CleanResult, ProjectCleaner, RustProject};
use std::sync::mpsc;
use std::thread;

/// 清理事件处理器
pub struct CleanHandler;

impl CleanHandler {
    /// 开始清理
    pub fn start_clean(
        selected_projects: Vec<RustProject>,
        config: CleanConfig,
        sender: mpsc::Sender<AppMessage>,
        stop_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) {
        thread::spawn(move || {
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
                        let _ = sender.send(AppMessage::CleanProjectComplete(
                            project.name.clone(),
                            size_freed,
                        ));
                        let _ = sender.send(AppMessage::CleanProgress(i + 1, total, total_freed));
                    }
                    Err(e) => {
                        let _ = sender.send(AppMessage::CleanError(format!(
                            "清理项目 {} 失败: {}",
                            project.name, e
                        )));
                        return;
                    }
                }
            }

            // 创建清理结果
            let mut result = CleanResult::new();
            result.cleaned_projects = selected_projects.len();
            result.total_size_freed = total_freed;

            if !stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                let _ = sender.send(AppMessage::CleanComplete(result));
            }
        });
    }
}
