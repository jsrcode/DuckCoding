use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::Level;
use tracing::level_filters::LevelFilter;

/// 日志级别枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => Level::ERROR,
            LogLevel::Warn => Level::WARN,
            LogLevel::Info => Level::INFO,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Trace => Level::TRACE,
        }
    }
}

impl From<Level> for LogLevel {
    fn from(level: Level) -> Self {
        match level {
            Level::ERROR => LogLevel::Error,
            Level::WARN => LogLevel::Warn,
            Level::INFO => LogLevel::Info,
            Level::DEBUG => LogLevel::Debug,
            Level::TRACE => LogLevel::Trace,
        }
    }
}

impl LogLevel {
    /// 转换为 tracing::Level
    pub fn to_tracing_level(self) -> tracing::Level {
        match self {
            LogLevel::Error => tracing::Level::ERROR,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Trace => tracing::Level::TRACE,
        }
    }

    /// 转换为 LevelFilter
    pub fn to_level_filter(self) -> LevelFilter {
        match self {
            LogLevel::Error => LevelFilter::ERROR,
            LogLevel::Warn => LevelFilter::WARN,
            LogLevel::Info => LevelFilter::INFO,
            LogLevel::Debug => LevelFilter::DEBUG,
            LogLevel::Trace => LevelFilter::TRACE,
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Error => write!(f, "error"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Trace => write!(f, "trace"),
        }
    }
}

/// 日志配置结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 日志级别
    pub level: LogLevel,
    /// 是否启用控制台输出
    pub console_enabled: bool,
    /// 是否启用文件输出
    pub file_enabled: bool,
    /// 自定义日志文件路径
    pub file_path: Option<PathBuf>,
    /// 是否使用JSON格式
    pub json_format: bool,
    /// 最大文件大小 (字节)
    pub max_file_size: Option<u64>,
    /// 日志文件保留天数
    pub max_files: Option<u32>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            console_enabled: true,
            file_enabled: true,
            file_path: None,
            json_format: false,
            max_file_size: Some(10 * 1024 * 1024), // 10MB
            max_files: Some(7), // 保留7天
        }
    }
}

impl LoggingConfig {
    /// 解析字符串为日志级别
    pub fn parse_level(level_str: &str) -> Result<LogLevel, String> {
        match level_str.to_lowercase().as_str() {
            "error" => Ok(LogLevel::Error),
            "warn" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            _ => Err(format!("无效的日志级别: {}", level_str)),
        }
    }

    /// 获取默认日志文件路径
    pub fn default_log_file_path() -> PathBuf {
        if let Some(home_dir) = dirs::data_dir() {
            home_dir.join("DuckCoding").join("logs")
        } else {
            std::env::temp_dir().join("DuckCoding").join("logs")
        }
    }

    /// 获取有效的日志文件路径
    pub fn get_effective_log_path(&self) -> PathBuf {
        self.file_path
            .clone()
            .unwrap_or_else(Self::default_log_file_path)
    }
}

/// 日志统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingStats {
    pub total_logs: u64,
    pub error_count: u64,
    pub warn_count: u64,
    pub info_count: u64,
    pub debug_count: u64,
    pub trace_count: u64,
    pub log_file_size: Option<u64>,
    pub uptime_seconds: u64,
}

impl Default for LoggingStats {
    fn default() -> Self {
        Self {
            total_logs: 0,
            error_count: 0,
            warn_count: 0,
            info_count: 0,
            debug_count: 0,
            trace_count: 0,
            log_file_size: None,
            uptime_seconds: 0,
        }
    }
}