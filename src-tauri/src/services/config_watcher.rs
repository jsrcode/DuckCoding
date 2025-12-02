use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};
use notify::{
    Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use serde::Serialize;
use tauri::Emitter;
use tracing::{debug, warn};

use crate::services::config::ConfigService;
use crate::services::profile_store::file_checksum;
use crate::Tool;

#[derive(Debug, Clone, Serialize)]
pub struct ExternalChange {
    pub tool_id: String,
    pub path: PathBuf,
    pub checksum: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub dirty: bool,
    pub fallback_poll: bool,
}

/// Tauri 事件名称（外部配置变更）
pub const EXTERNAL_CHANGE_EVENT: &str = "external-config-changed";

/// 简单的轮询 watcher，便于测试与跨平台复用；后续可替换为 OS 级通知。
pub struct ConfigWatcher {
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl ConfigWatcher {
    /// 轮询监听单文件变更。
    pub fn watch_file_polling(
        tool_id: impl Into<String>,
        path: PathBuf,
        poll_interval: Duration,
        mark_dirty: bool,
    ) -> Result<(Self, mpsc::Receiver<ExternalChange>)> {
        let tool_id = tool_id.into();
        let mut last_checksum = file_checksum(&path).ok();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_token = stop.clone();
        let (tx, rx) = mpsc::channel();
        let watch_path = path.clone();

        let handle = thread::spawn(move || {
            while !stop_token.load(Ordering::Relaxed) {
                let checksum = file_checksum(&watch_path).ok();
                if checksum.is_some() && checksum != last_checksum {
                    // 轻微防抖，避免写入过程中的空文件/瞬时内容导致重复事件
                    thread::sleep(Duration::from_millis(10));
                    let stable_checksum = file_checksum(&watch_path).ok().or(checksum.clone());

                    if stable_checksum.is_some() && stable_checksum != last_checksum {
                        last_checksum = stable_checksum.clone();
                        let change = ExternalChange {
                            tool_id: tool_id.clone(),
                            path: watch_path.clone(),
                            checksum: stable_checksum,
                            timestamp: Utc::now(),
                            dirty: mark_dirty,
                            fallback_poll: true,
                        };
                        let _ = tx.send(change);
                    }
                }
                thread::sleep(poll_interval);
            }
        });

        Ok((
            Self {
                stop,
                handle: Some(handle),
            },
            rx,
        ))
    }
}

/// 基于 notify 的实时 watcher，收到事件后写入 dirty 状态并广播到前端。
pub struct NotifyWatcherManager {
    _watchers: Vec<RecommendedWatcher>,
}

impl NotifyWatcherManager {
    fn watch_single(
        tool: Tool,
        path: PathBuf,
        app: tauri::AppHandle,
    ) -> Result<RecommendedWatcher> {
        let path_for_cb = path.clone();
        let tool_for_state = tool.clone();
        let mut last_checksum = ConfigService::compute_native_checksum(&tool_for_state);
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    match event.kind {
                        EventKind::Modify(_) | EventKind::Create(_) => {
                            let checksum = ConfigService::compute_native_checksum(&tool_for_state);
                            // 去重：相同 checksum 不重复触发
                            if checksum == last_checksum {
                                return;
                            }
                            last_checksum = checksum.clone();

                            match ConfigService::mark_external_change(
                                &tool_for_state,
                                path_for_cb.clone(),
                                checksum,
                            ) {
                                Ok(change) => {
                                    // 仅在确实变脏时通知前端，避免内部写入误报
                                    if change.dirty {
                                        debug!(
                                            tool = %change.tool_id,
                                            path = %change.path,
                                            checksum = ?change.checksum,
                                            "检测到配置文件改动（notify watcher）"
                                        );
                                        let _ = app.emit(EXTERNAL_CHANGE_EVENT, change);
                                    }
                                }
                                Err(err) => {
                                    warn!(
                                        tool = %tool_for_state.id,
                                        path = ?path_for_cb,
                                        error = ?err,
                                        "标记外部变更失败"
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
            },
            NotifyConfig::default(),
        )?;

        watcher.watch(&path, RecursiveMode::NonRecursive)?;
        Ok(watcher)
    }

    /// 为已存在的配置文件启动 watcher，方便 UI 实时感知。
    pub fn start_all(app: tauri::AppHandle) -> Result<Self> {
        let mut watchers = Vec::new();
        for tool in Tool::all() {
            let mut seen = HashSet::new();
            for path in ConfigService::config_paths(&tool) {
                if !seen.insert(path.clone()) {
                    continue;
                }
                if !path.exists() {
                    warn!(
                        tool = %tool.id,
                        path = ?path,
                        "配置文件不存在，跳过通知 watcher（将依赖轮询/手动刷新）"
                    );
                    continue;
                }
                let watcher = Self::watch_single(tool.clone(), path, app.clone())?;
                watchers.push(watcher);
            }
        }
        debug!(count = watchers.len(), "通知 watcher 启动完成");
        Ok(Self {
            _watchers: watchers,
        })
    }
}

impl Drop for ConfigWatcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::SystemTime;

    #[test]
    fn watcher_emits_on_change_and_filters_duplicate_checksum() -> Result<()> {
        let dir = std::env::temp_dir().join(format!(
            "duckcoding_watch_test_{}",
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        fs::create_dir_all(&dir)?;
        let path = dir.join("settings.json");
        fs::write(&path, r#"{"env":{"KEY":"A"}}"#)?;

        let (_watcher, rx) = ConfigWatcher::watch_file_polling(
            "claude-code",
            path.clone(),
            Duration::from_millis(50),
            true,
        )?;

        // 改变内容，期望收到事件
        fs::write(&path, r#"{"env":{"KEY":"B"}}"#)?;
        let change = rx
            .recv_timeout(Duration::from_secs(3))
            .expect("should receive change event");
        assert_eq!(change.tool_id, "claude-code");
        assert_eq!(change.path, path);
        assert!(change.checksum.is_some());

        // 再写入相同内容，不应再次触发
        fs::write(&path, r#"{"env":{"KEY":"B"}}"#)?;
        assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());

        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn watcher_respects_mark_dirty_flag() -> Result<()> {
        let dir = std::env::temp_dir().join(format!(
            "duckcoding_watch_test_mark_dirty_{}",
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        fs::create_dir_all(&dir)?;
        let path = dir.join("settings.json");
        fs::write(&path, r#"{"env":{"KEY":"X"}}"#)?;

        // mark_dirty = false，应当仍能收到事件，但 dirty 为 false
        let (_watcher, rx) = ConfigWatcher::watch_file_polling(
            "codex",
            path.clone(),
            Duration::from_millis(30),
            false,
        )?;

        fs::write(&path, r#"{"env":{"KEY":"Y"}}"#)?;
        let change = rx
            .recv_timeout(Duration::from_secs(3))
            .expect("should receive change event");

        assert_eq!(change.tool_id, "codex");
        assert_eq!(change.path, path);
        assert!(change.checksum.is_some());
        assert!(!change.dirty, "dirty flag should respect mark_dirty=false");
        assert!(
            change.fallback_poll,
            "polling watcher should mark fallback_poll"
        );

        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }
}
