use crate::logging::config::{LoggingConfig, LogLevel};
use crate::logging::dynamic_filter::{DynamicLogFilter, set_global_log_level as set_global_level};
use anyhow::{Context, Result};
use std::fs;
use std::sync::Arc;
use std::time::Instant;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer, Registry,
};

/// 日志级别映射表
const LEVEL_FILTER_VALUES: [(LevelFilter, u8); 5] = [
    (LevelFilter::ERROR, 1),
    (LevelFilter::WARN, 2),
    (LevelFilter::INFO, 3),
    (LevelFilter::DEBUG, 4),
    (LevelFilter::TRACE, 5),
];

/// 日志管理器，支持真正的动态级别控制
pub struct LogManager {
    pub config: LoggingConfig,
    pub start_time: Instant,
    _guard: Option<tracing_appender::non_blocking::WorkerGuard>,
    dynamic_filter: DynamicLogFilter,
}

impl LogManager {
    /// 初始化日志系统
    pub fn init() -> Result<Self> {
        let config = Self::load_config_from_env().unwrap_or_default();
        Self::init_with_config(config)
    }

    /// 使用指定配置初始化日志系统
    pub fn init_with_config(config: LoggingConfig) -> Result<Self> {
        // 确保日志目录存在
        if config.file_enabled {
            let log_path = config.get_effective_log_path();
            if let Some(parent) = log_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("无法创建日志目录: {:?}", parent))?;
            }
        }

        // 创建动态过滤器
        let dynamic_filter = DynamicLogFilter::new();

        // 设置初始日志级别
        set_global_level(config.level);

        // 创建订阅器注册表和层集合
        let registry = Registry::default();
        let mut layers = Vec::new();

        // 用于保存文件日志的guard，确保文件不会被关闭
        let mut file_guard: Option<tracing_appender::non_blocking::WorkerGuard> = None;

        // 添加动态过滤器层（必须在其他层之前）
        layers.push(dynamic_filter.clone().boxed());

        // 添加控制台日志层
        if config.console_enabled {
            layers.push(Self::create_console_layer(&config)?);
        }

        // 添加文件日志层（如果有文件输出）
        if config.file_enabled {
            let (layer, guard) = Self::create_file_layer_with_guard(&config)?;
            file_guard = Some(guard);
            layers.push(layer);
        }

        // 初始化订阅器，使用trace级别让动态过滤器控制实际的过滤
        let filter = EnvFilter::new("duckcoding=trace");
        let subscriber = registry.with(layers).with(filter);
        subscriber.init();

        tracing::info!(
            "日志系统初始化完成 - 级别: {}, 控制台: {}, 文件: {} (支持实时动态级别控制)",
            config.level,
            config.console_enabled,
            config.file_enabled
        );

        Ok(Self {
            config,
            start_time: Instant::now(),
            _guard: file_guard,
            dynamic_filter,
        })
    }

    /// 从环境变量加载配置
    fn load_config_from_env() -> Option<LoggingConfig> {
        let mut config = LoggingConfig::default();

        if let Ok(level_str) = std::env::var("RUST_LOG") {
            if let Ok(level) = LoggingConfig::parse_level(&level_str) {
                config.level = level;
            }
        }

        if let Ok(enabled) = std::env::var("DUCKCODING_LOG_CONSOLE") {
            config.console_enabled = enabled.parse().unwrap_or(true);
        }

        if let Ok(enabled) = std::env::var("DUCKCODING_LOG_FILE") {
            config.file_enabled = enabled.parse().unwrap_or(true);
        }

        if let Ok(path) = std::env::var("DUCKCODING_LOG_PATH") {
            config.file_path = Some(path.into());
        }

        if let Ok(json_fmt) = std::env::var("DUCKCODING_LOG_JSON") {
            config.json_format = json_fmt.parse().unwrap_or(false);
        }

        Some(config)
    }

    /// 创建控制台日志层
    fn create_console_layer(
        _config: &LoggingConfig,
    ) -> Result<Box<dyn Layer<Registry> + Send + Sync>> {
        use tracing_appender::non_blocking;

        let (non_blocking, _guard) = non_blocking(std::io::stdout());

        let layer = fmt::layer()
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_file(false)
            .with_line_number(false)
            .with_timer(tracing_subscriber::fmt::time::LocalTime::rfc_3339())
            .with_writer(non_blocking)
            .boxed();

        Ok(layer)
    }

    /// 创建文件日志层（带guard）
    fn create_file_layer_with_guard(
        config: &LoggingConfig,
    ) -> Result<(Box<dyn Layer<Registry> + Send + Sync>, tracing_appender::non_blocking::WorkerGuard)> {
        use tracing_appender::non_blocking;
        use tracing_appender::rolling;

        let log_path = config.get_effective_log_path();
        let file_appender = rolling::daily(log_path, "duckcoding.log");
        let (non_blocking, guard) = non_blocking(file_appender);

        let layer = if config.json_format {
            fmt::layer()
                .json()
                .with_target(true)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(true)
                .with_line_number(true)
                .with_timer(tracing_subscriber::fmt::time::LocalTime::rfc_3339())
                .with_writer(non_blocking)
                .boxed()
        } else {
            fmt::layer()
                .with_target(true)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(true)
                .with_line_number(true)
                .with_timer(tracing_subscriber::fmt::time::LocalTime::rfc_3339())
                .with_writer(non_blocking)
                .boxed()
        };

        Ok((layer, guard))
    }

    /// LevelFilter 转换为 u8
    fn level_filter_to_u8(filter: LevelFilter) -> u8 {
        for &(level_filter, value) in &LEVEL_FILTER_VALUES {
            if filter == level_filter {
                return value;
            }
        }
        3 // 默认 INFO
    }

    /// u8 转换为 LogLevel
    fn u8_to_log_level(value: u8) -> LogLevel {
        match value {
            1 => LogLevel::Error,
            2 => LogLevel::Warn,
            3 => LogLevel::Info,
            4 => LogLevel::Debug,
            5 => LogLevel::Trace,
            _ => LogLevel::Info,
        }
    }

    /// 动态更新日志配置
    pub fn update_config(&mut self, new_config: LoggingConfig) -> Result<()> {
        let level = new_config.level;
        set_global_level(level);
        self.config = new_config;

        tracing::info!("重新配置日志系统 - 新级别: {} (实时生效)", level);

        Ok(())
    }

    /// 获取当前日志级别
    pub fn get_current_level() -> LogLevel {
        use crate::logging::dynamic_filter::get_global_log_level;
        get_global_log_level()
    }

    /// 设置日志级别（实时生效）
    pub fn set_log_level(level: LogLevel) -> Result<()> {
        set_global_level(level);
        tracing::info!("日志级别已实时更新为: {}", level);
        Ok(())
    }

    /// 获取日志统计信息
    pub fn get_stats(&self) -> crate::logging::config::LoggingStats {
        let uptime_seconds = self.start_time.elapsed().as_secs();

        // 尝试获取日志文件大小
        let log_file_size = if self.config.file_enabled {
            let log_path = self.config.get_effective_log_path().join("duckcoding.log");
            if log_path.exists() {
                fs::metadata(log_path).ok().map(|m| m.len())
            } else {
                None
            }
        } else {
            None
        };

        crate::logging::config::LoggingStats {
            uptime_seconds,
            log_file_size,
            // 注意：在实际实现中，这些计数器应该通过自定义层来收集
            // 这里只是返回默认值
            ..Default::default()
        }
    }

    /// 刷新所有日志缓冲区
    pub fn flush(&self) {
        // tracing 会自动刷新，这里可以添加额外的刷新逻辑
        tracing::debug!("日志缓冲区已刷新");
    }
}

/// 全局日志管理器实例
static mut GLOBAL_LOG_MANAGER: Option<Arc<std::sync::Mutex<LogManager>>> = None;
static GLOBAL_LOG_MANAGER_INIT: std::sync::Once = std::sync::Once::new();

/// 获取全局日志管理器
pub fn get_global_log_manager() -> Option<Arc<std::sync::Mutex<LogManager>>> {
    unsafe {
        GLOBAL_LOG_MANAGER.as_ref().map(|m| m.clone())
    }
}

/// 动态更新全局日志级别（实时生效）
pub fn set_global_log_level(level: LogLevel) -> Result<()> {
    set_global_level(level);
    tracing::info!("✅ 全局日志级别已实时更新为: {}", level);
    Ok(())
}

/// 初始化全局日志系统
pub fn init_global_logger() -> Result<()> {
    let log_manager = LogManager::init()?;

    // 将全局日志管理器存储在静态变量中
    GLOBAL_LOG_MANAGER_INIT.call_once(|| {
        unsafe {
            GLOBAL_LOG_MANAGER = Some(Arc::new(std::sync::Mutex::new(log_manager)));
        }
    });

    Ok(())
}