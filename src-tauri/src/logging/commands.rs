use crate::logging::config::{LoggingConfig, LoggingStats};
use crate::logging::logger::{LogManager, get_global_log_manager, set_global_log_level};
use anyhow::Result;

/// 设置日志级别
#[tauri::command]
pub async fn set_log_level(level: String) -> Result<(), String> {
    let parsed_level = LoggingConfig::parse_level(&level)
        .map_err(|e| format!("无效的日志级别: {}", e))?;

    set_global_log_level(parsed_level)
        .map_err(|e| format!("设置日志级别失败: {}", e))?;

    tracing::info!("日志级别已通过命令设置为: {}", parsed_level);
    Ok(())
}

/// 获取当前日志级别
#[tauri::command]
pub async fn get_log_level() -> Result<String, String> {
    let current_level = LogManager::get_current_level();
    Ok(current_level.to_string())
}

/// 获取当前日志配置
#[tauri::command]
pub async fn get_log_config() -> Result<LoggingConfig, String> {
    let manager = get_global_log_manager()
        .ok_or_else(|| "日志管理器未初始化".to_string())?;

    let config = manager
        .lock()
        .map_err(|e| format!("无法获取日志管理器锁: {}", e))?
        .config
        .clone();

    Ok(config)
}

/// 更新日志配置
#[tauri::command]
pub async fn update_log_config(config: LoggingConfig) -> Result<(), String> {
    let manager = get_global_log_manager()
        .ok_or_else(|| "日志管理器未初始化".to_string())?;

    let mut manager_guard = manager
        .lock()
        .map_err(|e| format!("无法获取日志管理器锁: {}", e))?;

    manager_guard
        .update_config(config.clone())
        .map_err(|e| format!("更新日志配置失败: {}", e))?;

    tracing::info!("日志配置已更新 - 级别: {}, 控制台: {}, 文件: {}",
        config.level, config.console_enabled, config.file_enabled);

    Ok(())
}

/// 获取日志统计信息
#[tauri::command]
pub async fn get_log_stats() -> Result<LoggingStats, String> {
    let manager = get_global_log_manager()
        .ok_or_else(|| "日志管理器未初始化".to_string())?;

    let stats = manager
        .lock()
        .map_err(|e| format!("无法获取日志管理器锁: {}", e))?
        .get_stats();

    Ok(stats)
}

/// 刷新日志缓冲区
#[tauri::command]
pub async fn flush_logs() -> Result<(), String> {
    let manager = get_global_log_manager()
        .ok_or_else(|| "日志管理器未初始化".to_string())?;

    manager
        .lock()
        .map_err(|e| format!("无法获取日志管理器锁: {}", e))?
        .flush();

    tracing::debug!("日志已通过命令刷新");
    Ok(())
}

/// 获取可用的日志级别列表
#[tauri::command]
pub async fn get_available_log_levels() -> Result<Vec<String>, String> {
    Ok(vec![
        "error".to_string(),
        "warn".to_string(),
        "info".to_string(),
        "debug".to_string(),
        "trace".to_string(),
    ])
}

/// 测试日志输出
#[tauri::command]
pub async fn test_logging() -> Result<(), String> {
    tracing::error!("这是一条测试错误日志");
    tracing::warn!("这是一条测试警告日志");
    tracing::info!("这是一条测试信息日志");
    tracing::debug!("这是一条测试调试日志");
    tracing::trace!("这是一条测试跟踪日志");

    // 使用结构化字段
    tracing::info!(
        user_id = 12345,
        action = "test_logging",
        "测试日志功能完成"
    );

    Ok(())
}

/// 打开日志文件所在目录
#[tauri::command]
pub async fn open_log_directory() -> Result<(), String> {
    let config = LoggingConfig::default();
    let log_path = config.get_effective_log_path();

    use std::process::Command;

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&log_path)
            .spawn()
            .map_err(|e| format!("无法打开文件管理器: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&log_path)
            .spawn()
            .map_err(|e| format!("无法打开访达: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&log_path)
            .spawn()
            .map_err(|e| format!("无法打开文件管理器: {}", e))?;
    }

    tracing::info!("已打开日志目录: {:?}", log_path);
    Ok(())
}

/// 清理旧日志文件
#[tauri::command]
pub async fn cleanup_old_logs(days_to_keep: u32) -> Result<usize, String> {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    let config = LoggingConfig::default();
    let log_path = config.get_effective_log_path();

    if !log_path.exists() {
        return Ok(0);
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let cutoff_time = now - (days_to_keep as u64 * 24 * 60 * 60);
    let mut deleted_count = 0;

    let entries = fs::read_dir(&log_path)
        .map_err(|e| format!("无法读取日志目录: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("无法读取目录条目: {}", e))?;
        let path = entry.path();

        if path.is_file() {
            if let Ok(metadata) = fs::metadata(&path) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(modified_time) = modified.duration_since(UNIX_EPOCH) {
                        if modified_time.as_secs() < cutoff_time {
                            if let Err(e) = fs::remove_file(&path) {
                                tracing::warn!("无法删除旧日志文件 {:?}: {}", path, e);
                            } else {
                                deleted_count += 1;
                                tracing::info!("已删除旧日志文件: {:?}", path);
                            }
                        }
                    }
                }
            }
        }
    }

    tracing::info!("日志清理完成，删除了 {} 个文件", deleted_count);
    Ok(deleted_count)
}

/// 获取最近的日志条目
#[tauri::command]
pub async fn get_recent_logs(lines: usize) -> Result<Vec<String>, String> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let config = LoggingConfig::default();
    let log_file = config.get_effective_log_path().join("duckcoding.log");

    if !log_file.exists() {
        return Ok(vec!["日志文件不存在".to_string()]);
    }

    let file = File::open(&log_file)
        .map_err(|e| format!("无法打开日志文件: {}", e))?;

    let reader = BufReader::new(file);

    // 读取所有行到内存中
    let all_lines: Vec<String> = reader
        .lines()
        .filter_map(|line| line.ok())
        .collect();

    // 从末尾取指定行数
    let recent_logs: Vec<String> = all_lines
        .into_iter()
        .rev()
        .take(lines)
        .collect();

    // 反转回正确的时间顺序
    let mut log_lines = recent_logs;
    log_lines.reverse();

    Ok(log_lines)
}