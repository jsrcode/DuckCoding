//! DuckCoding 日志系统模块
//!
//! 提供结构化、异步、可配置的日志功能，支持：
//! - 动态级别控制
//! - 控制台和文件输出
//! - JSON格式可选
//! - 非阻塞异步处理
//! - Tauri命令集成

pub mod config;
pub mod logger;
pub mod commands;
pub mod dynamic_filter;

// 重新导出公共接口
pub use config::{LogLevel, LoggingConfig, LoggingStats};
pub use logger::{LogManager, init_global_logger, get_global_log_manager};

// 重新导出Tauri命令
pub use commands::{
    set_log_level,
    get_log_level,
    get_log_config,
    update_log_config,
    get_log_stats,
    flush_logs,
    get_available_log_levels,
    test_logging,
    open_log_directory,
    cleanup_old_logs,
    get_recent_logs,
};

/// 便捷宏：记录带有上下文的信息
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        tracing::info!($($arg)*);
    };
}

/// 便捷宏：记录带有上下文的警告
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        tracing::warn!($($arg)*);
    };
}

/// 便捷宏：记录带有上下文的错误
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        tracing::error!($($arg)*);
    };
}

/// 便捷宏：记录带有上下文的调试信息
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        tracing::debug!($($arg)*);
    };
}

/// 便捷宏：记录带有上下文的跟踪信息
#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        tracing::trace!($($arg)*);
    };
}