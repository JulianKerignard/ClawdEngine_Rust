use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

impl LogLevel {
    fn from_log(level: log::Level) -> Self {
        match level {
            log::Level::Error => Self::Error,
            log::Level::Warn => Self::Warn,
            log::Level::Info => Self::Info,
            log::Level::Debug | log::Level::Trace => Self::Debug,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub timestamp_secs: f64,
}

struct LogBufferInner {
    entries: std::collections::VecDeque<LogEntry>,
    start_time: Instant,
    max_entries: usize,
    error_count: u32,
    warn_count: u32,
    info_count: u32,
}

#[derive(Clone)]
pub struct LogBuffer {
    inner: Arc<Mutex<LogBufferInner>>,
}

impl LogBuffer {
    fn new(max_entries: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(LogBufferInner {
                entries: std::collections::VecDeque::with_capacity(256),
                start_time: Instant::now(),
                max_entries,
                error_count: 0,
                warn_count: 0,
                info_count: 0,
            })),
        }
    }

    fn push(&self, level: LogLevel, message: String) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let timestamp_secs = inner.start_time.elapsed().as_secs_f64();

        match level {
            LogLevel::Error => inner.error_count += 1,
            LogLevel::Warn => inner.warn_count += 1,
            LogLevel::Info => inner.info_count += 1,
            LogLevel::Debug => {}
        }

        if inner.entries.len() >= inner.max_entries {
            inner.entries.pop_front();
        }
        inner.entries.push_back(LogEntry {
            level,
            message,
            timestamp_secs,
        });
    }

    pub fn snapshot(&self) -> Vec<LogEntry> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entries
            .iter()
            .cloned()
            .collect()
    }

    pub fn clear(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.entries.clear();
        inner.error_count = 0;
        inner.warn_count = 0;
        inner.info_count = 0;
    }

    pub fn counts(&self) -> (u32, u32, u32) {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        (inner.error_count, inner.warn_count, inner.info_count)
    }
}

struct EngineLogger {
    buffer: LogBuffer,
    env_filter: env_logger::Logger,
}

impl log::Log for EngineLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Debug
    }

    fn log(&self, record: &log::Record) {
        if self.env_filter.enabled(record.metadata()) {
            self.env_filter.log(record);
        }

        if record.target().starts_with("clawdengine") || record.level() <= log::Level::Warn {
            self.buffer.push(
                LogLevel::from_log(record.level()),
                format!("{}", record.args()),
            );
        }
    }

    fn flush(&self) {
        self.env_filter.flush();
    }
}

pub fn init_logger() -> LogBuffer {
    let buffer = LogBuffer::new(1000);

    let env_logger = env_logger::Builder::from_default_env().build();
    let max_level = env_logger.filter();

    let logger = EngineLogger {
        buffer: buffer.clone(),
        env_filter: env_logger,
    };

    log::set_boxed_logger(Box::new(logger)).expect("Failed to set logger");
    log::set_max_level(max_level.max(log::LevelFilter::Debug));

    buffer
}
