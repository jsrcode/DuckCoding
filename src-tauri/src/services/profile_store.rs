use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::utils::config;

/// 集中配置中心目录结构：
/// - ~/.duckcoding/profiles/{tool}/{profile}.{ext}
/// - ~/.duckcoding/active/{tool}.json
/// - ~/.duckcoding/metadata/index.json
/// - 后续模块将基于这些路径进行统一读写与监听。
const PROFILES_DIR: &str = "profiles";
const ACTIVE_DIR: &str = "active";
const METADATA_DIR: &str = "metadata";
const INDEX_FILE: &str = "index.json";
const MIGRATION_LOG: &str = "migration.log.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProfileFormat {
    Json,
    Toml,
    Env,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProfileSource {
    #[default]
    Local,
    Imported,
    ExternalChange,
    Migrated,
    Generated,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActiveProfileState {
    pub profile_name: Option<String>,
    pub native_checksum: Option<String>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileDescriptor {
    pub tool_id: String,
    pub name: String,
    #[serde(default)]
    pub format: ProfileFormat,
    pub path: PathBuf,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub source: ProfileSource,
    #[serde(default)]
    pub checksum: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileIndex {
    /// tool_id -> descriptors
    pub entries: HashMap<String, Vec<ProfileDescriptor>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MigrationRecord {
    pub tool_id: String,
    pub profile_name: String,
    pub from_path: PathBuf,
    pub to_path: PathBuf,
    pub succeeded: bool,
    pub message: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// 返回集中配置根目录（确保存在）
pub fn center_root() -> Result<PathBuf> {
    let base = config::config_dir().map_err(|e| anyhow!(e))?;
    fs::create_dir_all(&base).context("创建 ~/.duckcoding 失败")?;
    Ok(base)
}

pub fn profiles_root() -> Result<PathBuf> {
    let root = center_root()?;
    let profiles = root.join(PROFILES_DIR);
    fs::create_dir_all(&profiles).context("创建 profiles 目录失败")?;
    Ok(profiles)
}

pub fn tool_profiles_dir(tool_id: &str) -> Result<PathBuf> {
    let dir = profiles_root()?.join(tool_id);
    fs::create_dir_all(&dir).with_context(|| format!("创建工具配置目录失败: {dir:?}"))?;
    Ok(dir)
}

pub fn active_state_path(tool_id: &str) -> Result<PathBuf> {
    let dir = center_root()?.join(ACTIVE_DIR);
    fs::create_dir_all(&dir).context("创建 active 目录失败")?;
    Ok(dir.join(format!("{tool_id}.json")))
}

pub fn metadata_index_path() -> Result<PathBuf> {
    let dir = center_root()?.join(METADATA_DIR);
    fs::create_dir_all(&dir).context("创建 metadata 目录失败")?;
    Ok(dir.join(INDEX_FILE))
}

pub fn migration_log_path() -> Result<PathBuf> {
    let dir = center_root()?.join(METADATA_DIR);
    fs::create_dir_all(&dir).context("创建 metadata 目录失败")?;
    Ok(dir.join(MIGRATION_LOG))
}

pub fn profile_file_path(tool_id: &str, profile_name: &str, ext: &str) -> Result<PathBuf> {
    let dir = tool_profiles_dir(tool_id)?;
    Ok(dir.join(format!("{profile_name}.{ext}")))
}

/// 计算文件的 sha256 哈希，便于自写过滤或外部更改检测。
pub fn file_checksum(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let content = fs::read(path).with_context(|| format!("读取文件失败: {path:?}"))?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let digest = hasher.finalize();
    Ok(format!("{digest:x}"))
}

pub fn read_migration_log() -> Result<Vec<MigrationRecord>> {
    let path = migration_log_path()?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = fs::read_to_string(&path)?;
    let records: Vec<MigrationRecord> = serde_json::from_str(&content)?;
    Ok(records)
}

fn profile_index_path() -> Result<PathBuf> {
    metadata_index_path()
}

fn load_index() -> Result<ProfileIndex> {
    let path = profile_index_path()?;
    if !path.exists() {
        return Ok(ProfileIndex::default());
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("读取元数据索引失败: {path:?}"))?;
    let index: ProfileIndex =
        serde_json::from_str(&content).with_context(|| format!("解析元数据索引失败: {path:?}"))?;
    Ok(index)
}

fn save_index(mut index: ProfileIndex) -> Result<()> {
    let path = profile_index_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    // 按 tool_id 排序，便于 diff
    let mut sorted_entries: Vec<_> = index.entries.into_iter().collect();
    sorted_entries.sort_by(|a, b| a.0.cmp(&b.0));
    index.entries = sorted_entries.into_iter().collect();
    let json = serde_json::to_string_pretty(&index).context("序列化元数据索引失败")?;
    fs::write(&path, json).with_context(|| format!("写入元数据索引失败: {path:?}"))?;
    Ok(())
}

fn upsert_descriptor(descriptor: ProfileDescriptor) -> Result<()> {
    let mut index = load_index()?;
    index
        .entries
        .entry(descriptor.tool_id.clone())
        .or_default()
        .retain(|p| p.name != descriptor.name);
    index
        .entries
        .entry(descriptor.tool_id.clone())
        .or_default()
        .push(descriptor);
    save_index(index)
}

fn remove_descriptor(tool_id: &str, profile_name: &str) -> Result<()> {
    let mut index = load_index()?;
    if let Some(list) = index.entries.get_mut(tool_id) {
        list.retain(|p| p.name != profile_name);
    }
    save_index(index)
}

/// 读取元数据索引（供外部调用）
pub fn load_profile_index() -> Result<ProfileIndex> {
    load_index()
}

/// 根据 tool_id 过滤描述，None 返回所有
pub fn list_descriptors(tool_id: Option<&str>) -> Result<Vec<ProfileDescriptor>> {
    let index = load_index()?;
    let mut all = Vec::new();
    match tool_id {
        Some(id) => {
            if let Some(list) = index.entries.get(id) {
                all.extend(list.clone());
            }
        }
        None => {
            for list in index.entries.values() {
                all.extend(list.clone());
            }
        }
    }
    Ok(all)
}

pub fn profile_extension(tool_id: &str) -> &'static str {
    match tool_id {
        "gemini-cli" => "json",
        "claude-code" => "json",
        "codex" => "json",
        _ => "json",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool_id", rename_all = "kebab-case")]
pub enum ProfilePayload {
    #[serde(rename = "claude-code")]
    Claude {
        api_key: String,
        base_url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        raw_settings: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        raw_config_json: Option<serde_json::Value>,
    },
    #[serde(rename = "codex")]
    Codex {
        api_key: String,
        base_url: String,
        provider: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        raw_config_toml: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        raw_auth_json: Option<serde_json::Value>,
    },
    #[serde(rename = "gemini-cli")]
    Gemini {
        api_key: String,
        base_url: String,
        model: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        raw_settings: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        raw_env: Option<String>,
    },
}

impl ProfilePayload {
    pub fn api_key(&self) -> &str {
        match self {
            ProfilePayload::Claude { api_key, .. } => api_key,
            ProfilePayload::Codex { api_key, .. } => api_key,
            ProfilePayload::Gemini { api_key, .. } => api_key,
        }
    }

    pub fn base_url(&self) -> &str {
        match self {
            ProfilePayload::Claude { base_url, .. } => base_url,
            ProfilePayload::Codex { base_url, .. } => base_url,
            ProfilePayload::Gemini { base_url, .. } => base_url,
        }
    }
}

pub fn save_profile_payload(
    tool_id: &str,
    profile_name: &str,
    payload: &ProfilePayload,
) -> Result<()> {
    let ext = profile_extension(tool_id);
    let path = profile_file_path(tool_id, profile_name, ext)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("创建配置目录失败: {parent:?}"))?;
    }

    let now = Utc::now();
    let checksum_source = serde_json::to_string(payload).context("序列化配置用于校验失败")?;
    let checksum = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(checksum_source.as_bytes());
        format!("{:x}", hasher.finalize())
    };

    let descriptor = ProfileDescriptor {
        tool_id: tool_id.to_string(),
        name: profile_name.to_string(),
        format: ProfileFormat::Json,
        path: path.clone(),
        created_at: Some(now),
        updated_at: Some(now),
        source: ProfileSource::Local,
        checksum: Some(checksum),
        tags: vec![],
    };

    let content = serde_json::to_string_pretty(payload).context("序列化配置失败")?;
    fs::write(&path, content).with_context(|| format!("写入集中配置失败: {path:?}"))?;

    upsert_descriptor(descriptor)?;
    Ok(())
}

pub fn load_profile_payload(tool_id: &str, profile_name: &str) -> Result<ProfilePayload> {
    let ext = profile_extension(tool_id);
    let path = profile_file_path(tool_id, profile_name, ext)?;
    let content = fs::read_to_string(&path).with_context(|| format!("读取配置失败: {path:?}"))?;
    let payload: ProfilePayload =
        serde_json::from_str(&content).with_context(|| format!("解析配置失败: {path:?}"))?;
    Ok(payload)
}

pub fn list_profile_names(tool_id: &str) -> Result<Vec<String>> {
    let dir = tool_profiles_dir(tool_id)?;
    let ext = profile_extension(tool_id);
    let mut profiles = Vec::new();
    if dir.exists() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let suffix = path.extension().and_then(|e| e.to_str());
            if suffix == Some(ext) {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    profiles.push(stem.to_string());
                }
            }
        }
    }
    profiles.sort();
    profiles.dedup();
    Ok(profiles)
}

pub fn delete_profile(tool_id: &str, profile_name: &str) -> Result<()> {
    let ext = profile_extension(tool_id);
    let path = profile_file_path(tool_id, profile_name, ext)?;
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("删除配置失败: {path:?}"))?;
    }
    remove_descriptor(tool_id, profile_name)?;
    Ok(())
}

pub fn read_active_state(tool_id: &str) -> Result<Option<ActiveProfileState>> {
    let path = active_state_path(tool_id)?;
    if !path.exists() {
        return Ok(None);
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("读取激活状态失败: {path:?}"))?;
    let state: ActiveProfileState =
        serde_json::from_str(&content).with_context(|| format!("解析激活状态失败: {path:?}"))?;
    Ok(Some(state))
}

pub fn save_active_state(tool_id: &str, state: &ActiveProfileState) -> Result<()> {
    let path = active_state_path(tool_id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(state).context("序列化激活状态失败")?;
    fs::write(&path, json).with_context(|| format!("写入激活状态失败: {path:?}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use serial_test::serial;
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempConfigGuard {
        path: PathBuf,
    }

    impl Drop for TempConfigGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
            env::remove_var("DUCKCODING_CONFIG_DIR");
        }
    }

    fn setup_temp_dir() -> Result<TempConfigGuard> {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let path = env::temp_dir().join(format!("duckcoding_test_{suffix}"));
        if path.exists() {
            fs::remove_dir_all(&path)?;
        }
        env::set_var("DUCKCODING_CONFIG_DIR", &path);
        Ok(TempConfigGuard { path })
    }

    #[test]
    #[serial]
    fn save_and_load_profile_payload_roundtrip() -> Result<()> {
        let _guard = setup_temp_dir()?;
        let tool_id = "claude-code";
        let profile = "roundtrip";

        let payload = ProfilePayload::Claude {
            api_key: "test-key".to_string(),
            base_url: "https://example.com".to_string(),
            raw_settings: None,
            raw_config_json: None,
        };

        save_profile_payload(tool_id, profile, &payload)?;
        let names = list_profile_names(tool_id)?;
        assert!(names.contains(&profile.to_string()));

        let loaded = load_profile_payload(tool_id, profile)?;
        match loaded {
            ProfilePayload::Claude {
                api_key, base_url, ..
            } => {
                assert_eq!(api_key, "test-key");
                assert_eq!(base_url, "https://example.com");
            }
            _ => panic!("unexpected payload variant"),
        }

        let state = ActiveProfileState {
            profile_name: Some(profile.to_string()),
            native_checksum: Some("abc".to_string()),
            last_synced_at: None,
            dirty: false,
        };
        save_active_state(tool_id, &state)?;
        let loaded_state = read_active_state(tool_id)?.expect("state should exist");
        assert_eq!(loaded_state.profile_name, state.profile_name);
        assert_eq!(loaded_state.native_checksum, state.native_checksum);

        Ok(())
    }

    #[test]
    #[serial]
    fn delete_profile_removes_file_and_descriptor() -> Result<()> {
        let _guard = setup_temp_dir()?;
        let tool_id = "codex";
        let profile = "temp";
        let payload = ProfilePayload::Codex {
            api_key: "k1".to_string(),
            base_url: "https://example.com".to_string(),
            provider: Some("duckcoding".to_string()),
            raw_config_toml: None,
            raw_auth_json: None,
        };

        save_profile_payload(tool_id, profile, &payload)?;
        assert!(list_profile_names(tool_id)?.contains(&profile.to_string()));

        delete_profile(tool_id, profile)?;
        assert!(list_profile_names(tool_id)?.is_empty());

        let descriptors = list_descriptors(Some(tool_id))?;
        assert!(
            descriptors.iter().all(|d| d.name != profile),
            "descriptor should be removed after delete"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn list_descriptors_and_index_roundtrip() -> Result<()> {
        let _guard = setup_temp_dir()?;
        let payload_claude = ProfilePayload::Claude {
            api_key: "a".to_string(),
            base_url: "https://a.com".to_string(),
            raw_settings: None,
            raw_config_json: None,
        };
        let payload_gemini = ProfilePayload::Gemini {
            api_key: "b".to_string(),
            base_url: "https://b.com".to_string(),
            model: "m".to_string(),
            raw_settings: None,
            raw_env: None,
        };
        save_profile_payload("claude-code", "p1", &payload_claude)?;
        save_profile_payload("gemini-cli", "g1", &payload_gemini)?;

        let descriptors = list_descriptors(None)?;
        assert_eq!(descriptors.len(), 2);
        assert!(descriptors
            .iter()
            .any(|d| d.tool_id == "claude-code" && d.name == "p1"));
        assert!(descriptors
            .iter()
            .any(|d| d.tool_id == "gemini-cli" && d.name == "g1"));

        let index = load_profile_index()?;
        assert!(index.entries.contains_key("claude-code"));
        assert!(index.entries.contains_key("gemini-cli"));
        Ok(())
    }

    #[test]
    #[serial]
    fn read_migration_log_returns_records() -> Result<()> {
        let _guard = setup_temp_dir()?;
        let log_path = migration_log_path()?;
        let records = vec![MigrationRecord {
            tool_id: "claude-code".to_string(),
            profile_name: "p1".to_string(),
            from_path: PathBuf::from("old"),
            to_path: PathBuf::from("new"),
            succeeded: true,
            message: None,
            timestamp: Utc::now(),
        }];
        fs::write(&log_path, serde_json::to_string(&records)?)?;

        let loaded = read_migration_log()?;
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].profile_name, "p1");
        Ok(())
    }
}
