use crate::logging::config::LogLevel;
use std::sync::atomic::{AtomicU8, Ordering};
use tracing::Subscriber;
use tracing_subscriber::{
    layer::{Context, Layer},
    registry::LookupSpan,
};

/// 全局日志级别控制器
static GLOBAL_LOG_LEVEL: AtomicU8 = AtomicU8::new(3); // 默认 INFO (3)

/// 日志级别转换为数字
fn log_level_to_number(level: LogLevel) -> u8 {
    match level {
        LogLevel::Error => 1,
        LogLevel::Warn => 2,
        LogLevel::Info => 3,
        LogLevel::Debug => 4,
        LogLevel::Trace => 5,
    }
}

/// 数字转换为日志级别
fn number_to_log_level(num: u8) -> tracing::Level {
    match num {
        1 => tracing::Level::ERROR,
        2 => tracing::Level::WARN,
        3 => tracing::Level::INFO,
        4 => tracing::Level::DEBUG,
        5 => tracing::Level::TRACE,
        _ => tracing::Level::INFO,
    }
}

/// 转换 tracing::Level 到数字
fn tracing_level_to_number(level: &tracing::Level) -> u8 {
    match *level {
        tracing::Level::ERROR => 1,
        tracing::Level::WARN => 2,
        tracing::Level::INFO => 3,
        tracing::Level::DEBUG => 4,
        tracing::Level::TRACE => 5,
    }
}

/// 动态日志过滤器 - 可以实时控制日志级别
#[derive(Clone)]
pub struct DynamicLogFilter;

impl DynamicLogFilter {
    pub fn new() -> Self {
        Self
    }

    /// 设置全局日志级别
    pub fn set_global_level(level: LogLevel) {
        let level_num = log_level_to_number(level);
        GLOBAL_LOG_LEVEL.store(level_num, Ordering::SeqCst);
    }

    /// 获取当前全局日志级别
    pub fn get_global_level() -> LogLevel {
        let level_num = GLOBAL_LOG_LEVEL.load(Ordering::SeqCst);
        match level_num {
            1 => LogLevel::Error,
            2 => LogLevel::Warn,
            3 => LogLevel::Info,
            4 => LogLevel::Debug,
            5 => LogLevel::Trace,
            _ => LogLevel::Info,
        }
    }

    /// 检查是否应该记录某个级别的日志
    pub fn should_log(&self, level: &tracing::Level) -> bool {
        let current_level_num = GLOBAL_LOG_LEVEL.load(Ordering::SeqCst);
        let request_level_num = tracing_level_to_number(level);
        request_level_num <= current_level_num
    }
}

impl Default for DynamicLogFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for DynamicLogFilter
where
    S: Subscriber,
    S: for<'span> LookupSpan<'span>,
{
    fn on_event(&self, _event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        // 这个方法在事件即将被记录时调用
        // 我们不需要在这里做任何特殊处理，因为enabled方法已经控制了过滤
    }

    fn enabled(&self, metadata: &tracing::Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        // 检查是否是duckcoding模块的日志
        if metadata.target().starts_with("duckcoding") {
            return self.should_log(metadata.level());
        }

        // 对于其他模块，使用默认的INFO级别
        matches!(metadata.level(), &tracing::Level::ERROR | &tracing::Level::WARN | &tracing::Level::INFO)
    }

    fn register_callsite(&self, metadata: &'static tracing::Metadata<'static>) -> tracing::subscriber::Interest {
        if metadata.target().starts_with("duckcoding") {
            if self.should_log(metadata.level()) {
                tracing::subscriber::Interest::always()
            } else {
                tracing::subscriber::Interest::never()
            }
        } else {
            // 对于其他模块，总是返回sometimes
            tracing::subscriber::Interest::sometimes()
        }
    }
}

/// 便捷函数：设置全局日志级别
pub fn set_global_log_level(level: LogLevel) {
    DynamicLogFilter::set_global_level(level);
}

/// 便捷函数：获取全局日志级别
pub fn get_global_log_level() -> LogLevel {
    DynamicLogFilter::get_global_level()
}