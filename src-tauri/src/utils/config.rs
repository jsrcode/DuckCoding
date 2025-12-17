use crate::data::DataManager;
use crate::GlobalConfig;
use std::fs;
use std::path::PathBuf;

/// DuckCoding 配置目录 (~/.duckcoding)，若不存在则创建
pub fn config_dir() -> Result<PathBuf, String> {
    if let Ok(override_dir) = std::env::var("DUCKCODING_CONFIG_DIR") {
        let path = PathBuf::from(override_dir);
        if !path.exists() {
            fs::create_dir_all(&path)
                .map_err(|e| format!("Failed to create config directory: {e}"))?;
        }
        return Ok(path);
    }

    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    let config_dir = home_dir.join(".duckcoding");
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config directory: {e}"))?;
    }
    Ok(config_dir)
}

/// 全局配置文件路径
pub fn global_config_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join("config.json"))
}

/// 读取全局配置（若文件不存在返回 Ok(None)）
pub fn read_global_config() -> Result<Option<GlobalConfig>, String> {
    let config_path = global_config_path()?;
    if !config_path.exists() {
        return Ok(None);
    }

    // 使用 DataManager 读取配置（无缓存模式，确保读取最新配置）
    let manager = DataManager::new();
    let config_value = manager
        .json_uncached()
        .read(&config_path)
        .map_err(|e| format!("Failed to read config: {e}"))?;

    let config: GlobalConfig =
        serde_json::from_value(config_value).map_err(|e| format!("Failed to parse config: {e}"))?;

    // 注意：迁移逻辑已移到 MigrationManager，在应用启动时统一执行

    Ok(Some(config))
}

/// 写入全局配置
///
/// 注意：此函数仅写入配置文件，不会自动应用代理设置。
/// 如需应用代理，请在写入后调用 `services::proxy::config::apply_global_proxy()`
pub fn write_global_config(config: &GlobalConfig) -> Result<(), String> {
    let config_path = global_config_path()?;

    // 使用 DataManager 写入配置（无缓存模式）
    let manager = DataManager::new();
    let config_value =
        serde_json::to_value(config).map_err(|e| format!("Failed to serialize config: {e}"))?;

    manager
        .json_uncached()
        .write(&config_path, &config_value)
        .map_err(|e| format!("Failed to write config: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn config_dir_respects_env_override() {
        let temp = TempDir::new().expect("create temp dir");
        env::set_var("DUCKCODING_CONFIG_DIR", temp.path());
        let dir = config_dir().expect("config_dir should succeed");
        assert_eq!(dir, temp.path());
        assert!(dir.exists());
        env::remove_var("DUCKCODING_CONFIG_DIR");
    }

    #[test]
    #[serial]
    fn config_dir_creates_when_missing() {
        // use random temp child path to ensure it does not exist
        let temp = TempDir::new().expect("create temp dir");
        let custom = temp.path().join("nested");
        env::set_var("DUCKCODING_CONFIG_DIR", &custom);
        let dir = config_dir().expect("config_dir should create path");
        assert!(dir.exists());
        assert!(dir.ends_with("nested"));
        env::remove_var("DUCKCODING_CONFIG_DIR");
    }
}
