use crate::state::{AppMessage, AppSettings};
use purger_core::{ProjectScanner, scanner::ScanConfig};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

/// Scan event handler
pub struct ScanHandler;

impl ScanHandler {
    /// Start scanning
    pub fn start_scan(
        path: PathBuf,
        max_depth: Option<usize>,
        settings: AppSettings,
        sender: mpsc::Sender<AppMessage>,
        stop_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) {
        thread::spawn(move || {
            let config = ScanConfig {
                max_depth,
                keep_days: settings.keep_days,
                ignore_paths: settings.ignore_paths.iter().map(PathBuf::from).collect(),
                lazy_size_calculation: true,
                ..Default::default()
            };

            // keep_size_mb 的筛选延后到 GUI 侧（先快扫、后补齐大小）

            let scanner = ProjectScanner::new(config);
            let _ = sender.send(AppMessage::ScanProgress(0, 0));

            let progress_sender = sender.clone();
            let on_found = move |found: usize| {
                let _ = progress_sender.send(AppMessage::ScanProgress(found, 0));
            };

            match scanner.scan_with_cancel_and_progress(
                &path,
                Some(stop_flag.as_ref()),
                Some(&on_found),
            ) {
                Ok(projects) => {
                    if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                        return;
                    }

                    let total = projects.len();
                    let _ = sender.send(AppMessage::ScanProgress(total, total));

                    if !stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                        let _ = sender.send(AppMessage::ScanComplete(projects));
                    }
                }
                Err(e) => {
                    if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                        return;
                    }
                    if !stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                        let _ = sender.send(AppMessage::ScanError(e.to_string()));
                    }
                }
            }
        });
    }

    /// Select a folder
    pub fn select_folder() -> Option<String> {
        rfd::FileDialog::new()
            .set_title("选择要扫描的文件夹")
            .pick_folder()
            .map(|path| path.to_string_lossy().to_string())
    }
}
