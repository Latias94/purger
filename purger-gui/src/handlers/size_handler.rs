use crate::state::AppMessage;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use walkdir::WalkDir;

pub struct SizeHandler;

impl SizeHandler {
    pub fn start_size_calculation(
        projects: Vec<(PathBuf, bool)>,
        sender: mpsc::Sender<AppMessage>,
        stop_flag: Arc<AtomicBool>,
    ) {
        thread::spawn(move || {
            let targets: Vec<_> = projects
                .into_iter()
                .filter(|(_, has_target)| *has_target)
                .map(|(path, _)| path)
                .collect();

            let total = targets.len();
            if total == 0 {
                let _ = sender.send(AppMessage::SizeProgress(0, 0));
                return;
            }

            let _ = sender.send(AppMessage::SizeProgress(0, total));

            for (i, project_path) in targets.into_iter().enumerate() {
                if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }

                let target_path = project_path.join("target");
                let size = calculate_dir_size(&target_path, &stop_flag);
                if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }

                let _ = sender.send(AppMessage::ProjectSizeUpdate(project_path, size));
                let _ = sender.send(AppMessage::SizeProgress(i + 1, total));
                thread::sleep(Duration::from_millis(2));
            }
        });
    }
}

fn calculate_dir_size(path: &PathBuf, stop_flag: &AtomicBool) -> u64 {
    if !path.exists() {
        return 0;
    }

    let mut total = 0u64;
    for entry in WalkDir::new(path).follow_links(false).into_iter() {
        if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
            return total;
        }

        let Ok(entry) = entry else {
            continue;
        };
        if !entry.file_type().is_file() {
            continue;
        }

        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        total = total.saturating_add(metadata.len());
    }

    total
}
