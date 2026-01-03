use crate::state::AppMessage;
use purger_core::{CleanResult, ProjectCleaner, RustProject, cleaner::CleanConfig};
use std::sync::mpsc;
use std::thread;

/// Cleaning event handler
pub struct CleanHandler;

impl CleanHandler {
    /// Start cleaning
    pub fn start_clean(
        selected_projects: Vec<RustProject>,
        config: CleanConfig,
        sender: mpsc::Sender<AppMessage>,
        stop_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) {
        thread::spawn(move || {
            let start_time = std::time::Instant::now();
            let cleaner = ProjectCleaner::new(config);
            let total = selected_projects.len();
            let mut total_freed = 0u64;
            let mut result = CleanResult::new();

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
                        result.add_success(size_freed);
                        let _ = sender.send(AppMessage::CleanProjectComplete(
                            project.name.clone(),
                            size_freed,
                        ));
                        let _ = sender.send(AppMessage::CleanProgress(i + 1, total, total_freed));
                    }
                    Err(e) => {
                        result.add_failure(project.name.clone());
                        let _ = sender.send(AppMessage::CleanProjectError(
                            project.name.clone(),
                            e.to_string(),
                        ));
                        let _ = sender.send(AppMessage::CleanProgress(i + 1, total, total_freed));
                        continue;
                    }
                }
            }

            result.total_size_freed = total_freed;
            result.duration_ms = start_time.elapsed().as_millis() as u64;

            if !stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                let _ = sender.send(AppMessage::CleanComplete(result));
            }
        });
    }
}
