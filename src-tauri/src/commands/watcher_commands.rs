use duckcoding::services::config_watcher::NotifyWatcherManager;
use duckcoding::utils::config::{read_global_config, write_global_config};
use tauri::AppHandle;
use tracing::{debug, error, warn};

use crate::ExternalWatcherState;

/// 获取当前监听状态
#[tauri::command]
pub async fn get_watcher_status(
    state: tauri::State<'_, ExternalWatcherState>,
) -> Result<bool, String> {
    let guard = state
        .manager
        .lock()
        .map_err(|e| format!("锁定 watcher 状态失败: {e}"))?;
    let running = guard.is_some();
    debug!(running, "Watcher status queried");
    Ok(running)
}

/// 按需开启监听
#[tauri::command]
pub async fn start_watcher_if_needed(
    app: AppHandle,
    state: tauri::State<'_, ExternalWatcherState>,
) -> Result<bool, String> {
    {
        let guard = state
            .manager
            .lock()
            .map_err(|e| format!("锁定 watcher 状态失败: {e}"))?;
        if guard.is_some() {
            debug!("Watcher already running, skip start");
            return Ok(true);
        }
    }

    // 检查全局配置是否允许
    if let Ok(Some(cfg)) = read_global_config() {
        if !cfg.external_watch_enabled {
            warn!("Global config disabled external watch, skip start");
            return Err("已在全局配置中关闭监听".to_string());
        }
        debug!(
            enabled = cfg.external_watch_enabled,
            poll_interval_ms = cfg.external_poll_interval_ms,
            "Watcher start check: config loaded"
        );
    }

    let manager = NotifyWatcherManager::start_all(app.clone()).map_err(|e| {
        error!(error = ?e, "Failed to start notify watchers");
        e.to_string()
    })?;
    let mut guard = state
        .manager
        .lock()
        .map_err(|e| format!("锁定 watcher 状态失败: {e}"))?;
    *guard = Some(manager);
    debug!("Watcher started and manager stored");
    Ok(true)
}

/// 停止监听
#[tauri::command]
pub async fn stop_watcher(state: tauri::State<'_, ExternalWatcherState>) -> Result<bool, String> {
    let mut guard = state
        .manager
        .lock()
        .map_err(|e| format!("锁定 watcher 状态失败: {e}"))?;
    if guard.is_none() {
        warn!("Stop watcher called but watcher not running");
        return Ok(false);
    }
    *guard = None;
    debug!("Watcher stopped and manager cleared");
    Ok(true)
}

/// 同步保存监听开关并尝试应用（开启时立即启动 watcher；关闭时停止）
#[tauri::command]
pub async fn save_watcher_settings(
    app: AppHandle,
    state: tauri::State<'_, ExternalWatcherState>,
    enabled: bool,
    poll_interval_ms: Option<u64>,
) -> Result<(), String> {
    let mut cfg = read_global_config()
        .map_err(|e| format!("读取全局配置失败: {e}"))?
        .ok_or_else(|| "全局配置不存在，无法保存监听设置".to_string())?;
    let old_enabled = cfg.external_watch_enabled;
    let old_interval = cfg.external_poll_interval_ms;
    cfg.external_watch_enabled = enabled;
    if let Some(interval) = poll_interval_ms {
        cfg.external_poll_interval_ms = interval;
    }
    write_global_config(&cfg).map_err(|e| format!("保存全局配置失败: {e}"))?;

    if enabled {
        debug!(
            enabled,
            poll_interval_ms = cfg.external_poll_interval_ms,
            old_enabled,
            old_interval,
            "Saving watcher settings: starting watcher"
        );
        let started = start_watcher_if_needed(app, state).await?;
        if !started {
            error!("Watcher should start but start_watcher_if_needed returned false");
            return Err("监听未能启动".to_string());
        }
    } else {
        debug!(
            enabled,
            poll_interval_ms = cfg.external_poll_interval_ms,
            old_enabled,
            old_interval,
            "Saving watcher settings: stopping watcher"
        );
        let _ = stop_watcher(state).await?;
    }

    Ok(())
}
