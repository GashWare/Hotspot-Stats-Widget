use chrono::Local;
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub tag: String,
    pub message: String,
}

pub struct AppLogger {
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
    file_path: Option<PathBuf>,
}

static LOGGER: OnceLock<AppLogger> = OnceLock::new();

impl AppLogger {
    pub fn global() -> &'static AppLogger {
        LOGGER.get_or_init(|| {
            let log_file = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|dir| dir.join("netmonitor.log")))
                .or_else(|| Some(PathBuf::from("netmonitor.log")));

            AppLogger {
                entries: Arc::new(Mutex::new(VecDeque::with_capacity(500))),
                file_path: log_file,
            }
        })
    }

    pub fn log(&self, level: LogLevel, tag: &str, msg: &str) {
        let ts = Local::now().format("%H:%M:%S%.3f").to_string();
        let entry = LogEntry {
            timestamp: ts.clone(),
            level: level.clone(),
            tag: tag.to_string(),
            message: msg.to_string(),
        };

        let level_str = match level {
            LogLevel::Info => "INFO",
            LogLevel::Success => "SUCCESS",
            LogLevel::Warning => "WARN",
            LogLevel::Error => "ERROR",
        };

        // 1. Console stdout
        println!("[{}] [{}] [{}] {}", ts, level_str, tag, msg);

        // 2. In-memory buffer for UI log viewer (cap at 400 entries)
        if let Ok(mut lock) = self.entries.lock() {
            if lock.len() >= 400 {
                lock.pop_front();
            }
            lock.push_back(entry);
        }

        // 3. Persistent disk log file (appends cleanly with flush)
        if let Some(ref path) = self.file_path {
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(f, "[{}] [{}] [{}] {}", ts, level_str, tag, msg);
            }
        }
    }

    pub fn get_entries(&self) -> Vec<LogEntry> {
        self.entries.lock().map(|l| l.iter().cloned().collect()).unwrap_or_default()
    }

    pub fn clear(&self) {
        if let Ok(mut l) = self.entries.lock() {
            l.clear();
        }
    }

    pub fn get_log_path(&self) -> String {
        self.file_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "netmonitor.log".to_string())
    }
}

pub fn log_info(tag: &str, msg: &str) {
    AppLogger::global().log(LogLevel::Info, tag, msg);
}

pub fn log_success(tag: &str, msg: &str) {
    AppLogger::global().log(LogLevel::Success, tag, msg);
}

pub fn log_warn(tag: &str, msg: &str) {
    AppLogger::global().log(LogLevel::Warning, tag, msg);
}

pub fn log_error(tag: &str, msg: &str) {
    AppLogger::global().log(LogLevel::Error, tag, msg);
}
