use crate::state::{AppMessage, AppSettings};
use purger_core::{ProjectScanner, scanner::ScanConfig};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

/// 扫描事件处理器
pub struct ScanHandler;

impl ScanHandler {
    /// 开始扫描
    pub fn start_scan(
        path: PathBuf,
        max_depth: Option<usize>,
        settings: AppSettings,
        sender: mpsc::Sender<AppMessage>,
        stop_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) {
        thread::spawn(move || {
            let mut config = ScanConfig {
                max_depth,
                keep_days: settings.keep_days,
                ignore_paths: settings.ignore_paths.iter().map(PathBuf::from).collect(),
                ..Default::default()
            };

            // 转换MB为字节
            if let Some(size_mb) = settings.keep_size_mb {
                config.keep_size = Some((size_mb * 1_000_000.0) as u64);
            }

            let scanner = ProjectScanner::new(config);

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

                    if settings.target_only {
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

    /// 选择文件夹
    pub fn select_folder() -> Option<String> {
        rfd::FileDialog::new()
            .set_title("选择要扫描的文件夹")
            .pick_folder()
            .map(|path| path.to_string_lossy().to_string())
    }
}
