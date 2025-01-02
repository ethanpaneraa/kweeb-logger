use anyhow::Result;
use directories::ProjectDirs;
use log::{LevelFilter, Log};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Mutex,
};

pub struct FileLogger {
    file: Mutex<File>,
}

impl FileLogger {
    pub fn init() -> Result<PathBuf> {
        let log_file_path = get_log_file_path()?;
        
        if let Some(dir) = log_file_path.parent() {
            std::fs::create_dir_all(dir)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)?;

        let logger = FileLogger {
            file: Mutex::new(file),
        };

        log::set_max_level(LevelFilter::Info);
        log::set_boxed_logger(Box::new(logger))?;

        Ok(log_file_path)
    }
}

impl Log for FileLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let mut file = self.file.lock().unwrap();
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            writeln!(
                file,
                "{} {} - {}",
                timestamp,
                record.level(),
                record.args()
            )
            .unwrap();
        }
    }

    fn flush(&self) {
        if let Ok(mut file) = self.file.lock() {
            file.flush().unwrap();
        }
    }
}

fn get_log_file_path() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "kweeb-logger", "logger")
        .ok_or_else(|| anyhow::anyhow!("Failed to get project directories"))?;

    Ok(proj_dirs.data_dir().join("kweeb-logger.log"))
}