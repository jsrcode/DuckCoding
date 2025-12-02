use crate::models::Tool;
use crate::services::migration::MigrationService;
use crate::services::profile_store::{
    delete_profile as delete_stored_profile, list_profile_names as list_stored_profiles,
    load_profile_payload, read_active_state, save_active_state, save_profile_payload,
    ActiveProfileState, ProfilePayload,
};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use toml;
use toml_edit::{DocumentMut, Item, Table};

#[derive(Serialize, Deserialize)]
pub struct CodexSettingsPayload {
    pub config: Value,
    #[serde(rename = "authToken")]
    pub auth_token: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSettingsPayload {
    pub settings: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_config: Option<Value>,
}

fn merge_toml_tables(target: &mut Table, source: &Table) {
    let keys_to_remove: Vec<String> = target
        .iter()
        .map(|(key, _)| key.to_string())
        .filter(|key| !source.contains_key(key))
        .collect();
    for key in keys_to_remove {
        target.remove(&key);
    }

    for (key, item) in source.iter() {
        match item {
            Item::Table(source_table) => {
                let needs_new_table = match target.get(key) {
                    Some(existing) => !existing.is_table(),
                    None => true,
                };

                if needs_new_table {
                    let mut new_table = Table::new();
                    new_table.set_implicit(source_table.is_implicit());
                    target.insert(key, Item::Table(new_table));
                }

                if let Some(target_item) = target.get_mut(key) {
                    if let Some(target_table) = target_item.as_table_mut() {
                        target_table.set_implicit(source_table.is_implicit());
                        merge_toml_tables(target_table, source_table);
                        continue;
                    }
                }

                target.insert(key, item.clone());
            }
            Item::Value(source_value) => {
                let mut updated = false;
                if let Some(existing_item) = target.get_mut(key) {
                    if let Some(existing_value) = existing_item.as_value_mut() {
                        let prefix = existing_value.decor().prefix().cloned();
                        let suffix = existing_value.decor().suffix().cloned();
                        *existing_value = source_value.clone();
                        let decor = existing_value.decor_mut();
                        decor.clear();
                        if let Some(pref) = prefix {
                            decor.set_prefix(pref);
                        }
                        if let Some(suf) = suffix {
                            decor.set_suffix(suf);
                        }
                        updated = true;
                    }
                }

                if !updated {
                    target.insert(key, Item::Value(source_value.clone()));
                }
            }
            _ => {
                target.insert(key, item.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{EnvVars, Tool};
    use crate::services::profile_store::{file_checksum, load_profile_payload};
    use serial_test::serial;
    use std::env;
    use std::fs;
    use tempfile::TempDir;

    struct TempEnvGuard {
        config_dir: Option<String>,
        home: Option<String>,
        userprofile: Option<String>,
    }

    impl TempEnvGuard {
        fn new(dir: &TempDir) -> Self {
            let config_dir = env::var("DUCKCODING_CONFIG_DIR").ok();
            let home = env::var("HOME").ok();
            let userprofile = env::var("USERPROFILE").ok();
            env::set_var("DUCKCODING_CONFIG_DIR", dir.path());
            env::set_var("HOME", dir.path());
            env::set_var("USERPROFILE", dir.path());
            Self {
                config_dir,
                home,
                userprofile,
            }
        }
    }

    impl Drop for TempEnvGuard {
        fn drop(&mut self) {
            match &self.config_dir {
                Some(val) => env::set_var("DUCKCODING_CONFIG_DIR", val),
                None => env::remove_var("DUCKCODING_CONFIG_DIR"),
            };
            match &self.home {
                Some(val) => env::set_var("HOME", val),
                None => env::remove_var("HOME"),
            };
            match &self.userprofile {
                Some(val) => env::set_var("USERPROFILE", val),
                None => env::remove_var("USERPROFILE"),
            };
        }
    }

    fn make_temp_tool(id: &str, config_file: &str, base: &TempDir) -> Tool {
        Tool {
            id: id.to_string(),
            name: format!("{id}-tool"),
            group_name: "test".to_string(),
            npm_package: "pkg".to_string(),
            check_command: "cmd".to_string(),
            config_dir: base.path().join(id),
            config_file: config_file.to_string(),
            env_vars: EnvVars {
                api_key: "API_KEY".to_string(),
                base_url: "BASE_URL".to_string(),
            },
            use_proxy_for_version_check: false,
        }
    }

    #[test]
    #[serial]
    fn mark_external_change_clears_dirty_when_checksum_unchanged() -> Result<()> {
        let temp = TempDir::new().expect("create temp dir");
        let _guard = TempEnvGuard::new(&temp);
        let tool = make_temp_tool("test-tool", "settings.json", &temp);
        fs::create_dir_all(&tool.config_dir)?;

        let first = ConfigService::mark_external_change(
            &tool,
            tool.config_dir.join(&tool.config_file),
            Some("abc".to_string()),
        )?;
        assert!(first.dirty);

        let second = ConfigService::mark_external_change(
            &tool,
            tool.config_dir.join(&tool.config_file),
            Some("abc".to_string()),
        )?;
        assert!(
            !second.dirty,
            "same checksum should not keep dirty flag true"
        );

        let state = read_active_state(&tool.id)?.expect("state should exist");
        assert_eq!(state.native_checksum, Some("abc".to_string()));
        assert!(!state.dirty);
        Ok(())
    }

    #[test]
    #[serial]
    fn mark_external_change_preserves_last_synced_at() -> Result<()> {
        let temp = TempDir::new().expect("create temp dir");
        let _guard = TempEnvGuard::new(&temp);
        let tool = make_temp_tool("test-tool-sync", "settings.json", &temp);
        fs::create_dir_all(&tool.config_dir)?;

        let original_time = Utc::now();
        let initial_state = ActiveProfileState {
            profile_name: Some("profile-a".to_string()),
            native_checksum: Some("old-checksum".to_string()),
            last_synced_at: Some(original_time),
            dirty: false,
        };
        save_active_state(&tool.id, &initial_state)?;

        let change = ConfigService::mark_external_change(
            &tool,
            tool.config_dir.join(&tool.config_file),
            Some("new-checksum".to_string()),
        )?;
        assert!(change.dirty, "checksum change should mark dirty");

        let state = read_active_state(&tool.id)?.expect("state should exist");
        assert_eq!(
            state.last_synced_at,
            Some(original_time),
            "detection should not move last_synced_at"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn import_external_change_for_codex_writes_profile_and_state() -> Result<()> {
        let temp = TempDir::new().expect("create temp dir");
        let _guard = TempEnvGuard::new(&temp);
        let tool = make_temp_tool("codex", "config.toml", &temp);
        fs::create_dir_all(&tool.config_dir)?;

        let config_path = tool.config_dir.join(&tool.config_file);
        fs::write(
            &config_path,
            r#"
model_provider = "duckcoding"
[model_providers.duckcoding]
base_url = "https://example.com/v1"
"#,
        )?;
        let auth_path = tool.config_dir.join("auth.json");
        fs::write(&auth_path, r#"{"OPENAI_API_KEY":"test-key"}"#)?;

        let result = ConfigService::import_external_change(&tool, "profile-a", false)?;
        assert_eq!(result.profile_name, "profile-a");
        assert!(!result.was_new);

        let payload = load_profile_payload("codex", "profile-a")?;
        match payload {
            ProfilePayload::Codex {
                api_key,
                base_url,
                provider,
                raw_config_toml,
                raw_auth_json,
            } => {
                assert_eq!(api_key, "test-key");
                assert_eq!(base_url, "https://example.com/v1");
                assert_eq!(provider, Some("duckcoding".to_string()));
                assert!(raw_config_toml.is_some());
                assert!(raw_auth_json.is_some());
            }
            other => panic!("unexpected payload variant: {:?}", other),
        }

        let state = read_active_state("codex")?.expect("active state should exist");
        assert_eq!(state.profile_name, Some("profile-a".to_string()));
        assert!(!state.dirty);
        Ok(())
    }

    #[test]
    #[serial]
    fn apply_config_persists_claude_profile_and_state() -> Result<()> {
        let temp = TempDir::new().expect("create temp dir");
        let _guard = TempEnvGuard::new(&temp);
        let tool = Tool::claude_code();

        ConfigService::apply_config(&tool, "k-1", "https://api.claude.com", Some("dev"))?;

        let settings_path = tool.config_dir.join(&tool.config_file);
        let content = fs::read_to_string(&settings_path)?;
        let json: Value = serde_json::from_str(&content)?;
        let env_obj = json
            .get("env")
            .and_then(|v| v.as_object())
            .expect("env exists");
        assert_eq!(
            env_obj.get("ANTHROPIC_AUTH_TOKEN").and_then(|v| v.as_str()),
            Some("k-1")
        );
        assert_eq!(
            env_obj.get("ANTHROPIC_BASE_URL").and_then(|v| v.as_str()),
            Some("https://api.claude.com")
        );

        let payload = load_profile_payload("claude-code", "dev")?;
        match payload {
            ProfilePayload::Claude {
                api_key,
                base_url,
                raw_settings,
                raw_config_json,
            } => {
                assert_eq!(api_key, "k-1");
                assert_eq!(base_url, "https://api.claude.com");
                assert!(raw_settings.is_some());
                assert!(raw_config_json.is_none());
            }
            _ => panic!("unexpected payload"),
        }

        let state = read_active_state("claude-code")?.expect("state exists");
        assert_eq!(state.profile_name, Some("dev".to_string()));
        assert!(!state.dirty);
        Ok(())
    }

    #[test]
    #[serial]
    fn detect_and_ack_external_change_updates_state() -> Result<()> {
        let temp = TempDir::new().expect("create temp dir");
        let _guard = TempEnvGuard::new(&temp);
        let tool = make_temp_tool("claude-code", "settings.json", &temp);
        fs::create_dir_all(&tool.config_dir)?;
        let path = tool.config_dir.join(&tool.config_file);
        fs::write(
            &path,
            r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"a","ANTHROPIC_BASE_URL":"https://a"}}"#,
        )?;
        let initial_checksum = file_checksum(&path).ok();
        save_active_state(
            &tool.id,
            &ActiveProfileState {
                profile_name: Some("default".to_string()),
                native_checksum: initial_checksum.clone(),
                last_synced_at: None,
                dirty: false,
            },
        )?;

        // modify file
        fs::write(
            &path,
            r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"b","ANTHROPIC_BASE_URL":"https://b"}}"#,
        )?;
        let changes = ConfigService::detect_external_changes()?;
        assert_eq!(changes.len(), 1);
        assert!(changes[0].dirty);

        let state_dirty = read_active_state(&tool.id)?.expect("state exists");
        assert!(state_dirty.dirty);

        ConfigService::acknowledge_external_change(&tool)?;
        let state_clean = read_active_state(&tool.id)?.expect("state exists");
        assert!(!state_clean.dirty);
        assert_ne!(state_clean.native_checksum, initial_checksum);
        Ok(())
    }

    #[test]
    #[serial]
    fn detect_external_changes_tracks_codex_auth_file() -> Result<()> {
        let temp = TempDir::new().expect("create temp dir");
        let _guard = TempEnvGuard::new(&temp);
        let tool = Tool::codex();

        fs::create_dir_all(&tool.config_dir)?;
        let config_path = tool.config_dir.join(&tool.config_file);
        let auth_path = tool.config_dir.join("auth.json");
        fs::write(
            &config_path,
            r#"model_provider = "duckcoding"
[model_providers.duckcoding]
base_url = "https://example.com/v1"
"#,
        )?;
        fs::write(&auth_path, r#"{"OPENAI_API_KEY":"old"}"#)?;

        let checksum = ConfigService::compute_native_checksum(&tool);
        save_active_state(
            &tool.id,
            &ActiveProfileState {
                profile_name: Some("default".to_string()),
                native_checksum: checksum,
                last_synced_at: None,
                dirty: false,
            },
        )?;

        // 仅修改 auth.json，应当被检测到
        fs::write(&auth_path, r#"{"OPENAI_API_KEY":"new"}"#)?;
        let changes = ConfigService::detect_external_changes()?;
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].tool_id, "codex");
        assert!(changes[0].dirty);
        Ok(())
    }

    #[test]
    #[serial]
    fn detect_external_changes_tracks_gemini_env_file() -> Result<()> {
        let temp = TempDir::new().expect("create temp dir");
        let _guard = TempEnvGuard::new(&temp);
        let tool = Tool::gemini_cli();

        fs::create_dir_all(&tool.config_dir)?;
        let settings_path = tool.config_dir.join(&tool.config_file);
        let env_path = tool.config_dir.join(".env");
        fs::write(&settings_path, r#"{"ide":{"enabled":true}}"#)?;
        fs::write(
            &env_path,
            "GEMINI_API_KEY=old\nGOOGLE_GEMINI_BASE_URL=https://g.com\nGEMINI_MODEL=gemini-2.5-pro\n",
        )?;

        let checksum = ConfigService::compute_native_checksum(&tool);
        save_active_state(
            &tool.id,
            &ActiveProfileState {
                profile_name: Some("default".to_string()),
                native_checksum: checksum,
                last_synced_at: None,
                dirty: false,
            },
        )?;

        fs::write(
            &env_path,
            "GEMINI_API_KEY=new\nGOOGLE_GEMINI_BASE_URL=https://g.com\nGEMINI_MODEL=gemini-2.5-pro\n",
        )?;

        let changes = ConfigService::detect_external_changes()?;
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].tool_id, "gemini-cli");
        assert!(changes[0].dirty);
        Ok(())
    }

    #[test]
    #[serial]
    fn detect_external_changes_tracks_claude_extra_config() -> Result<()> {
        let temp = TempDir::new().expect("create temp dir");
        let _guard = TempEnvGuard::new(&temp);
        let tool = Tool::claude_code();

        fs::create_dir_all(&tool.config_dir)?;
        let settings_path = tool.config_dir.join(&tool.config_file);
        let extra_path = tool.config_dir.join("config.json");
        fs::write(
            &settings_path,
            r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"a","ANTHROPIC_BASE_URL":"https://a"}}"#,
        )?;
        fs::write(&extra_path, r#"{"project":"duckcoding"}"#)?;

        let checksum = ConfigService::compute_native_checksum(&tool);
        save_active_state(
            &tool.id,
            &ActiveProfileState {
                profile_name: Some("default".to_string()),
                native_checksum: checksum,
                last_synced_at: None,
                dirty: false,
            },
        )?;

        fs::write(&extra_path, r#"{"project":"duckcoding-updated"}"#)?;
        let changes = ConfigService::detect_external_changes()?;
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].tool_id, "claude-code");
        assert!(changes[0].dirty);
        Ok(())
    }

    #[test]
    #[serial]
    fn apply_config_codex_sets_provider_and_auth() -> Result<()> {
        let temp = TempDir::new().expect("create temp dir");
        let _guard = TempEnvGuard::new(&temp);
        let tool = Tool::codex();

        ConfigService::apply_config(
            &tool,
            "codex-key",
            "https://duckcoding.example/v1",
            Some("main"),
        )?;

        let config_path = tool.config_dir.join(&tool.config_file);
        let toml_content = fs::read_to_string(&config_path)?;
        assert!(
            toml_content.contains("model_provider = \"duckcoding\"")
                || toml_content.contains("model_provider=\"duckcoding\"")
        );
        assert!(toml_content.contains("https://duckcoding.example/v1"));

        let auth_path = tool.config_dir.join("auth.json");
        let auth_content = fs::read_to_string(&auth_path)?;
        let auth_json: Value = serde_json::from_str(&auth_content)?;
        assert_eq!(
            auth_json.get("OPENAI_API_KEY").and_then(|v| v.as_str()),
            Some("codex-key")
        );

        let payload = load_profile_payload("codex", "main")?;
        match payload {
            ProfilePayload::Codex {
                api_key,
                base_url,
                provider,
                raw_config_toml,
                raw_auth_json,
            } => {
                assert_eq!(api_key, "codex-key");
                assert_eq!(base_url, "https://duckcoding.example/v1");
                assert_eq!(provider, Some("duckcoding".to_string()));
                assert!(raw_config_toml.is_some());
                assert!(raw_auth_json.is_some());
            }
            _ => panic!("unexpected payload"),
        }

        Ok(())
    }

    #[test]
    #[serial]
    fn save_claude_settings_writes_extra_config() -> Result<()> {
        let temp = TempDir::new().expect("create temp dir");
        let _guard = TempEnvGuard::new(&temp);
        let tool = Tool::claude_code();

        let settings = serde_json::json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "k-claude",
                "ANTHROPIC_BASE_URL": "https://claude.example"
            }
        });
        let extra = serde_json::json!({"project": "duckcoding"});

        ConfigService::save_claude_settings(&settings, Some(&extra))?;

        let extra_path = tool.config_dir.join("config.json");
        let saved_extra: Value = serde_json::from_str(&fs::read_to_string(&extra_path)?)?;
        assert_eq!(
            saved_extra.get("project").and_then(|v| v.as_str()),
            Some("duckcoding")
        );

        let payload = load_profile_payload("claude-code", "default")?;
        match payload {
            ProfilePayload::Claude {
                api_key,
                base_url,
                raw_settings,
                raw_config_json,
            } => {
                assert_eq!(api_key, "k-claude");
                assert_eq!(base_url, "https://claude.example");
                assert!(raw_settings.is_some());
                assert!(raw_config_json.is_some());
            }
            _ => panic!("unexpected payload"),
        }
        Ok(())
    }

    #[test]
    #[serial]
    fn apply_config_gemini_sets_model_and_env() -> Result<()> {
        let temp = TempDir::new().expect("create temp dir");
        let _guard = TempEnvGuard::new(&temp);
        let tool = Tool::gemini_cli();

        ConfigService::apply_config(&tool, "gem-key", "https://gem.com", Some("blue"))?;

        let env_path = tool.config_dir.join(".env");
        let env_content = fs::read_to_string(&env_path)?;
        assert!(env_content.contains("GEMINI_API_KEY=gem-key"));
        assert!(env_content.contains("GOOGLE_GEMINI_BASE_URL=https://gem.com"));
        assert!(env_content.contains("GEMINI_MODEL=gemini-2.5-pro"));

        let payload = load_profile_payload("gemini-cli", "blue")?;
        match payload {
            ProfilePayload::Gemini {
                api_key,
                base_url,
                model,
                raw_settings,
                raw_env,
            } => {
                assert_eq!(api_key, "gem-key");
                assert_eq!(base_url, "https://gem.com");
                assert_eq!(model, "gemini-2.5-pro");
                assert!(raw_settings.is_some());
                assert!(raw_env.is_some());
            }
            _ => panic!("unexpected payload"),
        }
        Ok(())
    }

    #[test]
    #[serial]
    fn delete_profile_marks_active_dirty_when_matching() -> Result<()> {
        let temp = TempDir::new().expect("create temp dir");
        let _guard = TempEnvGuard::new(&temp);
        let tool = Tool::claude_code();
        save_profile_payload(
            &tool.id,
            "temp",
            &ProfilePayload::Claude {
                api_key: "x".to_string(),
                base_url: "https://x".to_string(),
                raw_settings: None,
                raw_config_json: None,
            },
        )?;
        save_active_state(
            &tool.id,
            &ActiveProfileState {
                profile_name: Some("temp".to_string()),
                native_checksum: Some("old".to_string()),
                last_synced_at: None,
                dirty: false,
            },
        )?;

        ConfigService::delete_profile(&tool, "temp")?;
        let state = read_active_state(&tool.id)?.expect("state exists");
        assert!(state.dirty);
        assert!(state.profile_name.is_none());
        Ok(())
    }
}

fn set_table_value(table: &mut Table, key: &str, value: Item) {
    match value {
        Item::Value(new_value) => {
            if let Some(item) = table.get_mut(key) {
                if let Some(existing) = item.as_value_mut() {
                    *existing = new_value;
                    return;
                }
            }
            table.insert(key, Item::Value(new_value));
        }
        other => {
            table.insert(key, other);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GeminiEnvPayload {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

#[derive(Serialize, Deserialize)]
pub struct GeminiSettingsPayload {
    pub settings: Value,
    pub env: GeminiEnvPayload,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalConfigChange {
    pub tool_id: String,
    pub path: String,
    pub checksum: Option<String>,
    pub detected_at: chrono::DateTime<chrono::Utc>,
    pub dirty: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportExternalChangeResult {
    pub profile_name: String,
    pub was_new: bool,
    pub replaced: bool,
    pub before_checksum: Option<String>,
    pub checksum: Option<String>,
}
/// 配置服务
pub struct ConfigService;

impl ConfigService {
    /// 应用配置（增量更新）
    pub fn apply_config(
        tool: &Tool,
        api_key: &str,
        base_url: &str,
        profile_name: Option<&str>,
    ) -> Result<()> {
        MigrationService::run_if_needed();
        let payload = match tool.id.as_str() {
            "claude-code" => {
                Self::apply_claude_config(tool, api_key, base_url)?;
                let (raw_settings, raw_config_json) = Self::read_claude_raw(tool);
                ProfilePayload::Claude {
                    api_key: api_key.to_string(),
                    base_url: base_url.to_string(),
                    raw_settings,
                    raw_config_json,
                }
            }
            "codex" => {
                let provider = if base_url.contains("duckcoding") {
                    Some("duckcoding".to_string())
                } else {
                    Some("custom".to_string())
                };
                Self::apply_codex_config(tool, api_key, base_url, provider.as_deref())?;
                let (raw_config_toml, raw_auth_json) = Self::read_codex_raw(tool);
                ProfilePayload::Codex {
                    api_key: api_key.to_string(),
                    base_url: base_url.to_string(),
                    provider,
                    raw_config_toml,
                    raw_auth_json,
                }
            }
            "gemini-cli" => {
                Self::apply_gemini_config(tool, api_key, base_url, None)?;
                let (raw_settings, raw_env) = Self::read_gemini_raw(tool);
                ProfilePayload::Gemini {
                    api_key: api_key.to_string(),
                    base_url: base_url.to_string(),
                    model: "gemini-2.5-pro".to_string(),
                    raw_settings,
                    raw_env,
                }
            }
            _ => anyhow::bail!("未知工具: {}", tool.id),
        };

        let profile_to_save = profile_name.unwrap_or("default");
        Self::persist_payload_for_tool(tool, profile_to_save, &payload)?;

        Ok(())
    }

    /// Claude Code 配置
    fn apply_claude_config(tool: &Tool, api_key: &str, base_url: &str) -> Result<()> {
        let config_path = tool.config_dir.join(&tool.config_file);

        // 读取现有配置
        let mut settings = if config_path.exists() {
            let content = fs::read_to_string(&config_path).context("读取配置文件失败")?;
            serde_json::from_str::<Value>(&content).unwrap_or(Value::Object(Map::new()))
        } else {
            Value::Object(Map::new())
        };

        // 确保有 env 字段
        if !settings.is_object() {
            settings = serde_json::json!({});
        }

        let obj = settings.as_object_mut().unwrap();
        if !obj.contains_key("env") {
            obj.insert("env".to_string(), Value::Object(Map::new()));
        }

        // 只更新 API 相关字段
        let env = obj.get_mut("env").unwrap().as_object_mut().unwrap();
        env.insert(
            tool.env_vars.api_key.clone(),
            Value::String(api_key.to_string()),
        );
        env.insert(
            tool.env_vars.base_url.clone(),
            Value::String(base_url.to_string()),
        );

        // 确保目录存在
        fs::create_dir_all(&tool.config_dir)?;

        // 写入配置
        let json = serde_json::to_string_pretty(&settings)?;
        fs::write(&config_path, json)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&config_path)?;
            let mut perms = metadata.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&config_path, perms)?;
        }

        let env_obj = settings
            .get("env")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        let api_key = env_obj
            .get("ANTHROPIC_AUTH_TOKEN")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let base_url = env_obj
            .get("ANTHROPIC_BASE_URL")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let profile_name = Self::profile_name_for_sync(&tool.id);
        let (raw_settings, raw_config_json) = Self::read_claude_raw(tool);
        let payload = ProfilePayload::Claude {
            api_key,
            base_url,
            raw_settings,
            raw_config_json,
        };
        Self::persist_payload_for_tool(tool, &profile_name, &payload)?;

        Ok(())
    }

    /// CodeX 配置（使用 toml_edit 保留注释和格式）
    fn apply_codex_config(
        tool: &Tool,
        api_key: &str,
        base_url: &str,
        provider_override: Option<&str>,
    ) -> Result<()> {
        let config_path = tool.config_dir.join(&tool.config_file);
        let auth_path = tool.config_dir.join("auth.json");

        // 确保目录存在
        fs::create_dir_all(&tool.config_dir)?;

        // 读取现有 config.toml（使用 toml_edit 保留注释）
        let mut doc = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            content
                .parse::<toml_edit::DocumentMut>()
                .map_err(|err| anyhow!("解析 Codex config.toml 失败: {err}"))?
        } else {
            toml_edit::DocumentMut::new()
        };
        let root_table = doc.as_table_mut();

        // 判断 provider 类型
        let is_duckcoding = base_url.contains("duckcoding");
        let provider_key = provider_override.unwrap_or(if is_duckcoding {
            "duckcoding"
        } else {
            "custom"
        });

        // 只更新必要字段（保留用户自定义配置和注释）
        if !root_table.contains_key("model") {
            set_table_value(root_table, "model", toml_edit::value("gpt-5-codex"));
        }
        if !root_table.contains_key("model_reasoning_effort") {
            set_table_value(
                root_table,
                "model_reasoning_effort",
                toml_edit::value("high"),
            );
        }
        if !root_table.contains_key("network_access") {
            set_table_value(root_table, "network_access", toml_edit::value("enabled"));
        }

        // 更新 model_provider
        set_table_value(root_table, "model_provider", toml_edit::value(provider_key));

        let normalized_base = base_url.trim_end_matches('/');
        let base_url_with_v1 = if normalized_base.ends_with("/v1") {
            normalized_base.to_string()
        } else {
            format!("{normalized_base}/v1")
        };

        // 增量更新 model_providers 表
        if !root_table
            .get("model_providers")
            .map(|item| item.is_table())
            .unwrap_or(false)
        {
            let mut table = toml_edit::Table::new();
            table.set_implicit(false);
            root_table.insert("model_providers", toml_edit::Item::Table(table));
        }

        let providers_table = root_table
            .get_mut("model_providers")
            .and_then(|item| item.as_table_mut())
            .ok_or_else(|| anyhow!("解析 codex 配置失败：model_providers 不是表结构"))?;

        if !providers_table.contains_key(provider_key) {
            let mut table = toml_edit::Table::new();
            table.set_implicit(false);
            providers_table.insert(provider_key, toml_edit::Item::Table(table));
        }

        if let Some(provider_table) = providers_table
            .get_mut(provider_key)
            .and_then(|item| item.as_table_mut())
        {
            provider_table.insert("name", toml_edit::value(provider_key));
            provider_table.insert("base_url", toml_edit::value(base_url_with_v1));
            provider_table.insert("wire_api", toml_edit::value("responses"));
            provider_table.insert("requires_openai_auth", toml_edit::value(true));
        } else {
            anyhow::bail!("解析 codex 配置失败：无法写入 model_providers.{provider_key}");
        }

        // 写入 config.toml（保留注释和格式）
        fs::write(&config_path, doc.to_string())?;

        // 更新 auth.json（增量）
        let mut auth_data = if auth_path.exists() {
            let content = fs::read_to_string(&auth_path)?;
            serde_json::from_str::<Value>(&content).unwrap_or(Value::Object(Map::new()))
        } else {
            Value::Object(Map::new())
        };

        if let Value::Object(ref mut auth_obj) = auth_data {
            auth_obj.insert(
                "OPENAI_API_KEY".to_string(),
                Value::String(api_key.to_string()),
            );
        }

        fs::write(&auth_path, serde_json::to_string_pretty(&auth_data)?)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in [&config_path, &auth_path] {
                if path.exists() {
                    let metadata = fs::metadata(path)?;
                    let mut perms = metadata.permissions();
                    perms.set_mode(0o600);
                    fs::set_permissions(path, perms)?;
                }
            }
        }

        Ok(())
    }

    /// Gemini CLI 配置
    fn apply_gemini_config(
        tool: &Tool,
        api_key: &str,
        base_url: &str,
        model_override: Option<&str>,
    ) -> Result<()> {
        let env_path = tool.config_dir.join(".env");
        let settings_path = tool.config_dir.join(&tool.config_file);

        // 确保目录存在
        fs::create_dir_all(&tool.config_dir)?;

        // 读取现有 .env
        let mut env_vars = HashMap::new();
        if env_path.exists() {
            let content = fs::read_to_string(&env_path)?;
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    if let Some((key, value)) = trimmed.split_once('=') {
                        env_vars.insert(key.trim().to_string(), value.trim().to_string());
                    }
                }
            }
        }

        // 更新 API 相关字段
        env_vars.insert("GOOGLE_GEMINI_BASE_URL".to_string(), base_url.to_string());
        env_vars.insert("GEMINI_API_KEY".to_string(), api_key.to_string());
        let model_value = model_override
            .map(|m| m.to_string())
            .or_else(|| env_vars.get("GEMINI_MODEL").cloned())
            .unwrap_or_else(|| "gemini-2.5-pro".to_string());
        env_vars.insert("GEMINI_MODEL".to_string(), model_value);

        // 写入 .env
        let env_content: Vec<String> = env_vars.iter().map(|(k, v)| format!("{k}={v}")).collect();
        fs::write(&env_path, env_content.join("\n") + "\n")?;

        // 读取并更新 settings.json
        let mut settings = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            serde_json::from_str::<Value>(&content).unwrap_or(Value::Object(Map::new()))
        } else {
            Value::Object(Map::new())
        };

        if let Value::Object(ref mut obj) = settings {
            if !obj.contains_key("ide") {
                obj.insert("ide".to_string(), serde_json::json!({"enabled": true}));
            }
            if !obj.contains_key("security") {
                obj.insert(
                    "security".to_string(),
                    serde_json::json!({
                        "auth": {"selectedType": "gemini-api-key"}
                    }),
                );
            }
        }

        fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in [&env_path, &settings_path] {
                if path.exists() {
                    let metadata = fs::metadata(path)?;
                    let mut perms = metadata.permissions();
                    perms.set_mode(0o600);
                    fs::set_permissions(path, perms)?;
                }
            }
        }

        Ok(())
    }

    /// 保存备份配置
    pub fn save_backup(tool: &Tool, profile_name: &str) -> Result<()> {
        match tool.id.as_str() {
            "claude-code" => Self::backup_claude(tool, profile_name)?,
            "codex" => Self::backup_codex(tool, profile_name)?,
            "gemini-cli" => Self::backup_gemini(tool, profile_name)?,
            _ => anyhow::bail!("未知工具: {}", tool.id),
        }
        Ok(())
    }

    fn backup_claude(tool: &Tool, profile_name: &str) -> Result<()> {
        let config_path = tool.config_dir.join(&tool.config_file);
        let backup_path = tool.backup_path(profile_name);

        if !config_path.exists() {
            anyhow::bail!("配置文件不存在，无法备份");
        }

        // 读取当前配置，只提取 API 相关字段
        let content = fs::read_to_string(&config_path).context("读取配置文件失败")?;
        let settings: Value = serde_json::from_str(&content).context("解析配置文件失败")?;

        // 只保存 API 相关字段
        let backup_data = serde_json::json!({
            "ANTHROPIC_AUTH_TOKEN": settings
                .get("env")
                .and_then(|env| env.get("ANTHROPIC_AUTH_TOKEN"))
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "ANTHROPIC_BASE_URL": settings
                .get("env")
                .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
        });

        // 写入备份（仅包含 API 字段）
        fs::write(&backup_path, serde_json::to_string_pretty(&backup_data)?)?;

        Ok(())
    }

    fn backup_codex(tool: &Tool, profile_name: &str) -> Result<()> {
        let config_path = tool.config_dir.join("config.toml");
        let auth_path = tool.config_dir.join("auth.json");

        let backup_config = tool.config_dir.join(format!("config.{profile_name}.toml"));
        let backup_auth = tool.config_dir.join(format!("auth.{profile_name}.json"));

        // 读取 auth.json 中的 API Key
        let api_key = if auth_path.exists() {
            let content = fs::read_to_string(&auth_path)?;
            let auth: Value = serde_json::from_str(&content)?;
            auth.get("OPENAI_API_KEY")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        } else {
            String::new()
        };

        // 只保存 API 相关字段到备份
        let backup_auth_data = serde_json::json!({
            "OPENAI_API_KEY": api_key
        });
        fs::write(
            &backup_auth,
            serde_json::to_string_pretty(&backup_auth_data)?,
        )?;

        // 对于 config.toml，只备份当前使用的 provider 的完整配置
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            if let Ok(doc) = content.parse::<toml_edit::DocumentMut>() {
                let mut backup_doc = toml_edit::DocumentMut::new();

                // 获取当前使用的 model_provider
                let current_provider_name = doc
                    .get("model_provider")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("配置文件缺少 model_provider 字段"))?;

                // 只备份当前 provider 的完整配置
                if let Some(providers) = doc.get("model_providers").and_then(|p| p.as_table()) {
                    if let Some(current_provider) = providers.get(current_provider_name) {
                        tracing::debug!(
                            provider = %current_provider_name,
                            profile = %profile_name,
                            "备份 Codex 配置"

                        );
                        let mut backup_providers = toml_edit::Table::new();
                        backup_providers.insert(current_provider_name, current_provider.clone());
                        backup_doc
                            .insert("model_providers", toml_edit::Item::Table(backup_providers));
                    } else {
                        anyhow::bail!("未找到 model_provider '{current_provider_name}' 的配置");
                    }
                } else {
                    anyhow::bail!("配置文件缺少 model_providers 表");
                }

                // 保存当前的 model_provider 选择
                backup_doc.insert("model_provider", toml_edit::value(current_provider_name));

                fs::write(&backup_config, backup_doc.to_string())?;
            }
        }

        Ok(())
    }

    fn backup_gemini(tool: &Tool, profile_name: &str) -> Result<()> {
        let env_path = tool.config_dir.join(".env");
        let backup_env = tool.config_dir.join(format!(".env.{profile_name}"));

        if !env_path.exists() {
            anyhow::bail!("配置文件不存在，无法备份");
        }

        // 读取 .env 文件，只提取 API 相关字段
        let content = fs::read_to_string(&env_path)?;
        let mut api_key = String::new();
        let mut base_url = String::new();
        let mut model = String::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = trimmed.split_once('=') {
                match key.trim() {
                    "GEMINI_API_KEY" => api_key = value.trim().to_string(),
                    "GOOGLE_GEMINI_BASE_URL" => base_url = value.trim().to_string(),
                    "GEMINI_MODEL" => model = value.trim().to_string(),
                    _ => {}
                }
            }
        }

        // 只保存 API 相关字段
        let backup_content = format!(
            "GEMINI_API_KEY={api_key}\nGOOGLE_GEMINI_BASE_URL={base_url}\nGEMINI_MODEL={model}\n"
        );

        fs::write(&backup_env, backup_content)?;

        Ok(())
    }

    /// 列出所有保存的配置
    pub fn list_profiles(tool: &Tool) -> Result<Vec<String>> {
        MigrationService::run_if_needed();
        list_stored_profiles(&tool.id)
    }

    /// 激活指定的配置
    pub fn activate_profile(tool: &Tool, profile_name: &str) -> Result<()> {
        MigrationService::run_if_needed();
        let payload = load_profile_payload(&tool.id, profile_name)?;
        match (tool.id.as_str(), payload) {
            (
                "claude-code",
                ProfilePayload::Claude {
                    api_key,
                    base_url,
                    raw_settings,
                    raw_config_json,
                },
            ) => {
                fs::create_dir_all(&tool.config_dir)?;
                let settings_path = tool.config_dir.join(&tool.config_file);
                let extra_config_path = tool.config_dir.join("config.json");

                match (raw_settings, raw_config_json) {
                    (Some(settings), extra) => {
                        fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
                        if let Some(cfg) = extra {
                            fs::write(&extra_config_path, serde_json::to_string_pretty(&cfg)?)?;
                        }
                    }
                    _ => {
                        Self::apply_claude_config(tool, &api_key, &base_url)?;
                    }
                }

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    for path in [&settings_path, &extra_config_path] {
                        if path.exists() {
                            let metadata = fs::metadata(path)?;
                            let mut perms = metadata.permissions();
                            perms.set_mode(0o600);
                            fs::set_permissions(path, perms)?;
                        }
                    }
                }
            }
            (
                "codex",
                ProfilePayload::Codex {
                    api_key,
                    base_url,
                    provider,
                    raw_config_toml,
                    raw_auth_json,
                },
            ) => {
                fs::create_dir_all(&tool.config_dir)?;
                let config_path = tool.config_dir.join(&tool.config_file);
                let auth_path = tool.config_dir.join("auth.json");

                if let Some(raw) = raw_config_toml {
                    fs::write(&config_path, raw)?;
                } else {
                    Self::apply_codex_config(tool, &api_key, &base_url, provider.as_deref())?;
                }

                if let Some(auth) = raw_auth_json {
                    fs::write(&auth_path, serde_json::to_string_pretty(&auth)?)?;
                }

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    for path in [&config_path, &auth_path] {
                        if path.exists() {
                            let metadata = fs::metadata(path)?;
                            let mut perms = metadata.permissions();
                            perms.set_mode(0o600);
                            fs::set_permissions(path, perms)?;
                        }
                    }
                }
            }
            (
                "gemini-cli",
                ProfilePayload::Gemini {
                    api_key,
                    base_url,
                    model,
                    raw_settings,
                    raw_env,
                },
            ) => {
                fs::create_dir_all(&tool.config_dir)?;
                let settings_path = tool.config_dir.join(&tool.config_file);
                let env_path = tool.config_dir.join(".env");

                match (raw_settings, raw_env) {
                    (Some(settings), Some(env_raw)) => {
                        fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
                        fs::write(&env_path, env_raw)?;
                    }
                    (Some(settings), None) => {
                        fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
                        let mut pairs = HashMap::new();
                        pairs.insert("GEMINI_API_KEY".to_string(), api_key.clone());
                        pairs.insert("GOOGLE_GEMINI_BASE_URL".to_string(), base_url.clone());
                        pairs.insert("GEMINI_MODEL".to_string(), model.clone());
                        Self::write_env_pairs(&env_path, &pairs)?;
                    }
                    (None, Some(env_raw)) => {
                        fs::write(&env_path, env_raw)?;
                        let settings = Value::Object(Map::new());
                        fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
                    }
                    (None, None) => {
                        // 缺失原始快照时回退到当前逻辑，保证核心字段存在
                        Self::apply_gemini_config(tool, &api_key, &base_url, Some(&model))?;
                    }
                }

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    for path in [&settings_path, &env_path] {
                        if path.exists() {
                            let metadata = fs::metadata(path)?;
                            let mut perms = metadata.permissions();
                            perms.set_mode(0o600);
                            fs::set_permissions(path, perms)?;
                        }
                    }
                }
            }
            _ => anyhow::bail!("配置内容与工具不匹配: {}", tool.id),
        }

        let checksum = Self::compute_native_checksum(tool);
        let state = ActiveProfileState {
            profile_name: Some(profile_name.to_string()),
            native_checksum: checksum,
            last_synced_at: Some(Utc::now()),
            dirty: false,
        };
        save_active_state(&tool.id, &state)?;
        Ok(())
    }

    /// 删除配置
    pub fn delete_profile(tool: &Tool, profile_name: &str) -> Result<()> {
        MigrationService::run_if_needed();
        delete_stored_profile(&tool.id, profile_name)?;
        if let Ok(Some(mut state)) = read_active_state(&tool.id) {
            if state.profile_name.as_deref() == Some(profile_name) {
                state.profile_name = None;
                state.dirty = true;
                state.last_synced_at = Some(Utc::now());
                let _ = save_active_state(&tool.id, &state);
            }
        }
        Ok(())
    }

    /// 读取 Claude Code 完整配置
    pub fn read_claude_settings() -> Result<Value> {
        let tool = Tool::claude_code();
        let config_path = tool.config_dir.join(&tool.config_file);

        if !config_path.exists() {
            return Ok(Value::Object(Map::new()));
        }

        let content = fs::read_to_string(&config_path).context("读取 Claude Code 配置失败")?;
        if content.trim().is_empty() {
            return Ok(Value::Object(Map::new()));
        }

        let settings: Value = serde_json::from_str(&content)
            .map_err(|err| anyhow!("解析 Claude Code 配置失败: {err}"))?;

        Ok(settings)
    }

    /// 读取 Claude Code 附属 config.json
    pub fn read_claude_extra_config() -> Result<Value> {
        let tool = Tool::claude_code();
        let extra_path = tool.config_dir.join("config.json");
        if !extra_path.exists() {
            return Ok(Value::Object(Map::new()));
        }
        let content =
            fs::read_to_string(&extra_path).context("读取 Claude Code config.json 失败")?;
        if content.trim().is_empty() {
            return Ok(Value::Object(Map::new()));
        }
        let json: Value = serde_json::from_str(&content)
            .map_err(|err| anyhow!("解析 Claude Code config.json 失败: {err}"))?;
        Ok(json)
    }

    /// 保存 Claude Code 完整配置
    pub fn save_claude_settings(settings: &Value, extra_config: Option<&Value>) -> Result<()> {
        if !settings.is_object() {
            anyhow::bail!("Claude Code 配置必须是 JSON 对象");
        }

        let tool = Tool::claude_code();
        let config_dir = &tool.config_dir;
        let config_path = config_dir.join(&tool.config_file);
        let extra_config_path = config_dir.join("config.json");

        fs::create_dir_all(config_dir).context("创建 Claude Code 配置目录失败")?;
        let json = serde_json::to_string_pretty(settings)?;
        fs::write(&config_path, json).context("写入 Claude Code 配置失败")?;

        if let Some(extra) = extra_config {
            if !extra.is_object() {
                anyhow::bail!("Claude Code config.json 必须是 JSON 对象");
            }
            fs::write(&extra_config_path, serde_json::to_string_pretty(extra)?)
                .context("写入 Claude Code config.json 失败")?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&config_path)?;
            let mut perms = metadata.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&config_path, perms)?;

            if extra_config_path.exists() {
                let metadata = fs::metadata(&extra_config_path)?;
                let mut perms = metadata.permissions();
                perms.set_mode(0o600);
                fs::set_permissions(&extra_config_path, perms)?;
            }
        }

        let env_obj = settings
            .get("env")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        let api_key = env_obj
            .get("ANTHROPIC_AUTH_TOKEN")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let base_url = env_obj
            .get("ANTHROPIC_BASE_URL")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let profile_name = Self::profile_name_for_sync(&tool.id);
        let (raw_settings, raw_config_json) = Self::read_claude_raw(&tool);
        let payload = ProfilePayload::Claude {
            api_key,
            base_url,
            raw_settings,
            raw_config_json,
        };
        Self::persist_payload_for_tool(&tool, &profile_name, &payload)?;

        Ok(())
    }

    /// 获取内置的 Claude Code JSON Schema
    pub fn get_claude_schema() -> Result<Value> {
        static CLAUDE_SCHEMA: OnceCell<Value> = OnceCell::new();

        let schema = CLAUDE_SCHEMA.get_or_try_init(|| {
            let raw = include_str!("../../resources/claude_code_settings.schema.json");
            serde_json::from_str(raw).context("解析 Claude Code Schema 失败")
        })?;

        Ok(schema.clone())
    }

    /// 读取 Codex config.toml 和 auth.json
    pub fn read_codex_settings() -> Result<CodexSettingsPayload> {
        let tool = Tool::codex();
        let config_path = tool.config_dir.join(&tool.config_file);
        let auth_path = tool.config_dir.join("auth.json");

        let config_value = if config_path.exists() {
            let content =
                fs::read_to_string(&config_path).context("读取 Codex config.toml 失败")?;
            let toml_value: toml::Value = toml::from_str(&content)
                .map_err(|err| anyhow!("解析 Codex config.toml 失败: {err}"))?;
            serde_json::to_value(toml_value).context("转换 Codex config.toml 为 JSON 失败")?
        } else {
            Value::Object(Map::new())
        };

        let auth_token = if auth_path.exists() {
            let content = fs::read_to_string(&auth_path).context("读取 Codex auth.json 失败")?;
            let auth: Value = serde_json::from_str(&content)
                .map_err(|err| anyhow!("解析 Codex auth.json 失败: {err}"))?;
            auth.get("OPENAI_API_KEY")
                .and_then(|s| s.as_str().map(|s| s.to_string()))
        } else {
            None
        };

        Ok(CodexSettingsPayload {
            config: config_value,
            auth_token,
        })
    }

    /// 保存 Codex 配置和 auth.json
    pub fn save_codex_settings(config: &Value, auth_token: Option<String>) -> Result<()> {
        if !config.is_object() {
            anyhow::bail!("Codex 配置必须是对象结构");
        }

        let tool = Tool::codex();
        let config_path = tool.config_dir.join(&tool.config_file);
        let auth_path = tool.config_dir.join("auth.json");
        fs::create_dir_all(&tool.config_dir).context("创建 Codex 配置目录失败")?;
        let mut final_auth_token = auth_token.clone();

        let mut existing_doc = if config_path.exists() {
            let content =
                fs::read_to_string(&config_path).context("读取 Codex config.toml 失败")?;
            content
                .parse::<DocumentMut>()
                .map_err(|err| anyhow!("解析 Codex config.toml 失败: {err}"))?
        } else {
            DocumentMut::new()
        };

        let new_toml_string = toml::to_string(config).context("序列化 Codex config 失败")?;
        let new_doc = new_toml_string
            .parse::<DocumentMut>()
            .map_err(|err| anyhow!("解析待写入 Codex 配置失败: {err}"))?;

        merge_toml_tables(existing_doc.as_table_mut(), new_doc.as_table());

        fs::write(&config_path, existing_doc.to_string()).context("写入 Codex config.toml 失败")?;

        if let Some(token) = auth_token {
            final_auth_token = Some(token.clone());
            let mut auth_data = if auth_path.exists() {
                let content = fs::read_to_string(&auth_path).unwrap_or_default();
                serde_json::from_str::<Value>(&content).unwrap_or(Value::Object(Map::new()))
            } else {
                Value::Object(Map::new())
            };

            if let Value::Object(ref mut obj) = auth_data {
                obj.insert("OPENAI_API_KEY".to_string(), Value::String(token));
            }

            fs::write(&auth_path, serde_json::to_string_pretty(&auth_data)?)
                .context("写入 Codex auth.json 失败")?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let metadata = fs::metadata(&auth_path)?;
                let mut perms = metadata.permissions();
                perms.set_mode(0o600);
                fs::set_permissions(&auth_path, perms)?;
            }
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&config_path)?;
            let mut perms = metadata.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&config_path, perms)?;
        }

        if final_auth_token.is_none() && auth_path.exists() {
            if let Ok(content) = fs::read_to_string(&auth_path) {
                if let Ok(auth) = serde_json::from_str::<Value>(&content) {
                    final_auth_token = auth
                        .get("OPENAI_API_KEY")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
            }
        }

        let provider_name = config
            .get("model_provider")
            .and_then(|v| v.as_str())
            .unwrap_or("duckcoding")
            .to_string();
        let base_url = config
            .get("model_providers")
            .and_then(|v| v.as_object())
            .and_then(|providers| providers.get(&provider_name))
            .and_then(|provider| provider.get("base_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("https://jp.duckcoding.com/v1")
            .to_string();

        let api_key_for_store = final_auth_token.unwrap_or_default();
        let profile_name = Self::profile_name_for_sync(&tool.id);
        let (raw_config_toml, raw_auth_json) = Self::read_codex_raw(&tool);
        let payload = ProfilePayload::Codex {
            api_key: api_key_for_store,
            base_url: base_url.clone(),
            provider: Some(provider_name),
            raw_config_toml,
            raw_auth_json,
        };
        Self::persist_payload_for_tool(&tool, &profile_name, &payload)?;

        Ok(())
    }

    /// 获取 Codex config schema
    pub fn get_codex_schema() -> Result<Value> {
        static CODEX_SCHEMA: OnceCell<Value> = OnceCell::new();
        let schema = CODEX_SCHEMA.get_or_try_init(|| {
            let raw = include_str!("../../resources/codex_config.schema.json");
            serde_json::from_str(raw).context("解析 Codex Schema 失败")
        })?;

        Ok(schema.clone())
    }

    /// 读取 Gemini CLI 配置与 .env
    pub fn read_gemini_settings() -> Result<GeminiSettingsPayload> {
        let tool = Tool::gemini_cli();
        let settings_path = tool.config_dir.join(&tool.config_file);
        let env_path = tool.config_dir.join(".env");

        let settings = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path).context("读取 Gemini CLI 配置失败")?;
            if content.trim().is_empty() {
                Value::Object(Map::new())
            } else {
                serde_json::from_str(&content)
                    .map_err(|err| anyhow!("解析 Gemini CLI 配置失败: {err}"))?
            }
        } else {
            Value::Object(Map::new())
        };

        let env = Self::read_gemini_env(&env_path)?;

        Ok(GeminiSettingsPayload { settings, env })
    }

    /// 保存 Gemini CLI 配置与 .env
    pub fn save_gemini_settings(settings: &Value, env: &GeminiEnvPayload) -> Result<()> {
        if !settings.is_object() {
            anyhow::bail!("Gemini CLI 配置必须是 JSON 对象");
        }

        let tool = Tool::gemini_cli();
        let config_dir = &tool.config_dir;
        let settings_path = config_dir.join(&tool.config_file);
        let env_path = config_dir.join(".env");
        fs::create_dir_all(config_dir).context("创建 Gemini CLI 配置目录失败")?;

        let json = serde_json::to_string_pretty(settings)?;
        fs::write(&settings_path, json).context("写入 Gemini CLI 配置失败")?;

        let mut env_pairs = Self::read_env_pairs(&env_path)?;
        env_pairs.insert("GEMINI_API_KEY".to_string(), env.api_key.clone());
        env_pairs.insert("GOOGLE_GEMINI_BASE_URL".to_string(), env.base_url.clone());
        env_pairs.insert(
            "GEMINI_MODEL".to_string(),
            if env.model.trim().is_empty() {
                "gemini-2.5-pro".to_string()
            } else {
                env.model.clone()
            },
        );
        Self::write_env_pairs(&env_path, &env_pairs).context("写入 Gemini CLI .env 失败")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in [&settings_path, &env_path] {
                if path.exists() {
                    let metadata = fs::metadata(path)?;
                    let mut perms = metadata.permissions();
                    perms.set_mode(0o600);
                    fs::set_permissions(path, perms)?;
                }
            }
        }

        let profile_name = Self::profile_name_for_sync(&tool.id);
        let (raw_settings, raw_env) = Self::read_gemini_raw(&tool);
        let payload = ProfilePayload::Gemini {
            api_key: env.api_key.clone(),
            base_url: env.base_url.clone(),
            model: if env.model.trim().is_empty() {
                "gemini-2.5-pro".to_string()
            } else {
                env.model.clone()
            },
            raw_settings,
            raw_env,
        };
        Self::persist_payload_for_tool(&tool, &profile_name, &payload)?;

        Ok(())
    }

    /// 获取 Gemini CLI JSON Schema
    pub fn get_gemini_schema() -> Result<Value> {
        static GEMINI_SCHEMA: OnceCell<Value> = OnceCell::new();
        let schema = GEMINI_SCHEMA.get_or_try_init(|| {
            let raw = include_str!("../../resources/gemini_cli_settings.schema.json");
            serde_json::from_str(raw).context("解析 Gemini CLI Schema 失败")
        })?;

        Ok(schema.clone())
    }

    fn read_gemini_env(path: &Path) -> Result<GeminiEnvPayload> {
        if !path.exists() {
            return Ok(GeminiEnvPayload {
                model: "gemini-2.5-pro".to_string(),
                ..GeminiEnvPayload::default()
            });
        }

        let env_pairs = Self::read_env_pairs(path)?;
        Ok(GeminiEnvPayload {
            api_key: env_pairs.get("GEMINI_API_KEY").cloned().unwrap_or_default(),
            base_url: env_pairs
                .get("GOOGLE_GEMINI_BASE_URL")
                .cloned()
                .unwrap_or_default(),
            model: env_pairs
                .get("GEMINI_MODEL")
                .cloned()
                .unwrap_or_else(|| "gemini-2.5-pro".to_string()),
        })
    }

    fn read_env_pairs(path: &Path) -> Result<HashMap<String, String>> {
        let mut pairs = HashMap::new();
        if path.exists() {
            let content = fs::read_to_string(path)?;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = trimmed.split_once('=') {
                    pairs.insert(key.trim().to_string(), value.trim().to_string());
                }
            }
        }
        Ok(pairs)
    }

    fn write_env_pairs(path: &Path, pairs: &HashMap<String, String>) -> Result<()> {
        let mut items: Vec<_> = pairs.iter().collect();
        items.sort_by(|a, b| a.0.cmp(b.0));
        let mut content = String::new();
        for (idx, (key, value)) in items.iter().enumerate() {
            if idx > 0 {
                content.push('\n');
            }
            content.push_str(key);
            content.push('=');
            content.push_str(value);
        }
        content.push('\n');
        fs::write(path, content)?;
        Ok(())
    }

    fn read_codex_raw(tool: &Tool) -> (Option<String>, Option<Value>) {
        let config_path = tool.config_dir.join(&tool.config_file);
        let auth_path = tool.config_dir.join("auth.json");
        let raw_config_toml = fs::read_to_string(&config_path).ok();
        let raw_auth_json = fs::read_to_string(&auth_path)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok());
        (raw_config_toml, raw_auth_json)
    }

    fn read_claude_raw(tool: &Tool) -> (Option<Value>, Option<Value>) {
        let settings_path = tool.config_dir.join(&tool.config_file);
        let extra_config_path = tool.config_dir.join("config.json");
        let raw_settings = fs::read_to_string(&settings_path)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok());
        let raw_config_json = fs::read_to_string(&extra_config_path)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok());
        (raw_settings, raw_config_json)
    }

    fn read_gemini_raw(tool: &Tool) -> (Option<Value>, Option<String>) {
        let settings_path = tool.config_dir.join(&tool.config_file);
        let env_path = tool.config_dir.join(".env");
        let raw_settings = fs::read_to_string(&settings_path)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok());
        let raw_env = fs::read_to_string(&env_path).ok();
        (raw_settings, raw_env)
    }

    fn profile_name_for_sync(tool_id: &str) -> String {
        read_active_state(tool_id)
            .ok()
            .flatten()
            .and_then(|state| state.profile_name)
            .unwrap_or_else(|| "default".to_string())
    }

    fn persist_payload_for_tool(
        tool: &Tool,
        profile_name: &str,
        payload: &ProfilePayload,
    ) -> Result<()> {
        save_profile_payload(&tool.id, profile_name, payload)?;
        let checksum = Self::compute_native_checksum(tool);
        let state = ActiveProfileState {
            profile_name: Some(profile_name.to_string()),
            native_checksum: checksum,
            last_synced_at: Some(Utc::now()),
            dirty: false,
        };
        save_active_state(&tool.id, &state)?;
        Ok(())
    }

    /// 返回参与同步/监听的配置文件列表（包含主配置和附属文件）。
    pub(crate) fn config_paths(tool: &Tool) -> Vec<std::path::PathBuf> {
        let mut paths = vec![tool.config_dir.join(&tool.config_file)];
        match tool.id.as_str() {
            "codex" => {
                paths.push(tool.config_dir.join("auth.json"));
            }
            "gemini-cli" => {
                paths.push(tool.config_dir.join(".env"));
            }
            "claude-code" => {
                paths.push(tool.config_dir.join("config.json"));
            }
            _ => {}
        }
        paths
    }

    /// 计算配置文件组合哈希，任一文件变动都会改变结果。
    pub(crate) fn compute_native_checksum(tool: &Tool) -> Option<String> {
        use sha2::{Digest, Sha256};
        let mut paths = Self::config_paths(tool);
        paths.sort();

        let mut hasher = Sha256::new();
        let mut any_exists = false;
        for path in paths {
            hasher.update(path.to_string_lossy().as_bytes());
            if path.exists() {
                any_exists = true;
                match fs::read(&path) {
                    Ok(content) => hasher.update(&content),
                    Err(_) => return None,
                }
            } else {
                hasher.update(b"MISSING");
            }
        }

        if any_exists {
            Some(format!("{:x}", hasher.finalize()))
        } else {
            None
        }
    }

    fn read_payload_from_native(tool: &Tool) -> Result<ProfilePayload> {
        match tool.id.as_str() {
            "claude-code" => {
                let settings = Self::read_claude_settings()?;
                let env = settings
                    .get("env")
                    .and_then(|v| v.as_object())
                    .ok_or_else(|| anyhow!("配置缺少 env 字段"))?;
                let api_key = env
                    .get("ANTHROPIC_AUTH_TOKEN")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let base_url = env
                    .get("ANTHROPIC_BASE_URL")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();

                if api_key.is_empty() || base_url.is_empty() {
                    anyhow::bail!("原生配置缺少 API Key 或 Base URL");
                }

                let extra_config = fs::read_to_string(tool.config_dir.join("config.json"))
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok());

                Ok(ProfilePayload::Claude {
                    api_key,
                    base_url,
                    raw_settings: Some(settings),
                    raw_config_json: extra_config,
                })
            }
            "codex" => {
                let config_path = tool.config_dir.join(&tool.config_file);
                if !config_path.exists() {
                    anyhow::bail!("未找到原生配置文件: {:?}", config_path);
                }
                let content = fs::read_to_string(&config_path)
                    .with_context(|| format!("读取 Codex 配置失败: {config_path:?}"))?;
                let raw_config_toml = Some(content.clone());
                let toml_value: toml::Value =
                    toml::from_str(&content).context("解析 Codex config.toml 失败")?;

                let mut provider = toml_value
                    .get("model_provider")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let mut base_url = String::new();
                if let Some(providers) =
                    toml_value.get("model_providers").and_then(|v| v.as_table())
                {
                    if provider.is_none() {
                        provider = providers.keys().next().cloned();
                    }
                    if let Some(provider_name) = provider.clone() {
                        if let Some(toml::Value::Table(table)) = providers.get(&provider_name) {
                            if let Some(toml::Value::String(url)) = table.get("base_url") {
                                base_url = url.clone();
                            }
                        }
                    }
                }
                if base_url.is_empty() {
                    base_url = "https://jp.duckcoding.com/v1".to_string();
                }

                let auth_path = tool.config_dir.join("auth.json");
                let mut api_key = String::new();
                let mut raw_auth_json = None;
                if auth_path.exists() {
                    let auth_content =
                        fs::read_to_string(&auth_path).context("读取 Codex auth.json 失败")?;
                    let auth: Value =
                        serde_json::from_str(&auth_content).context("解析 Codex auth.json 失败")?;
                    raw_auth_json = Some(auth.clone());
                    api_key = auth
                        .get("OPENAI_API_KEY")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                }
                if api_key.is_empty() {
                    anyhow::bail!("auth.json 缺少 OPENAI_API_KEY");
                }

                Ok(ProfilePayload::Codex {
                    api_key,
                    base_url,
                    provider,
                    raw_config_toml,
                    raw_auth_json,
                })
            }
            "gemini-cli" => {
                let env_path = tool.config_dir.join(".env");
                if !env_path.exists() {
                    anyhow::bail!("未找到 Gemini CLI .env 配置: {:?}", env_path);
                }
                let raw_env = fs::read_to_string(&env_path).ok();
                let env_pairs = Self::read_env_pairs(&env_path)?;
                let api_key = env_pairs
                    .get("GEMINI_API_KEY")
                    .cloned()
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                let mut base_url = env_pairs
                    .get("GOOGLE_GEMINI_BASE_URL")
                    .cloned()
                    .unwrap_or_default();
                let model = env_pairs
                    .get("GEMINI_MODEL")
                    .cloned()
                    .unwrap_or_else(|| "gemini-2.5-pro".to_string());

                if base_url.trim().is_empty() {
                    base_url = "https://generativelanguage.googleapis.com".to_string();
                }
                if api_key.is_empty() {
                    anyhow::bail!(".env 缺少 GEMINI_API_KEY");
                }

                Ok(ProfilePayload::Gemini {
                    api_key,
                    base_url,
                    model,
                    raw_settings: fs::read_to_string(tool.config_dir.join(&tool.config_file))
                        .ok()
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    raw_env,
                })
            }
            other => anyhow::bail!("暂不支持的工具: {other}"),
        }
    }

    /// 将外部修改导入集中仓，并刷新激活状态。
    pub fn import_external_change(
        tool: &Tool,
        profile_name: &str,
        as_new: bool,
    ) -> Result<ImportExternalChangeResult> {
        MigrationService::run_if_needed();

        let target_profile = profile_name.trim();
        if target_profile.is_empty() {
            anyhow::bail!("profile 名称不能为空");
        }
        let existing = list_stored_profiles(&tool.id)?;
        let exists = existing.iter().any(|p| p == target_profile);
        if as_new && exists {
            anyhow::bail!("profile 已存在: {target_profile}");
        }

        let payload = Self::read_payload_from_native(tool)?;
        let checksum_before = Self::compute_native_checksum(tool);
        save_profile_payload(&tool.id, target_profile, &payload)?;

        let checksum = Self::compute_native_checksum(tool);
        let replaced = !as_new && exists;
        let state = ActiveProfileState {
            profile_name: Some(target_profile.to_string()),
            native_checksum: checksum.clone(),
            last_synced_at: Some(Utc::now()),
            dirty: false,
        };
        save_active_state(&tool.id, &state)?;

        Ok(ImportExternalChangeResult {
            profile_name: target_profile.to_string(),
            was_new: as_new,
            replaced,
            before_checksum: checksum_before,
            checksum,
        })
    }

    /// 扫描原生配置是否被外部修改，返回差异列表，并将 dirty 标记写入 active_state。
    pub fn detect_external_changes() -> Result<Vec<ExternalConfigChange>> {
        let mut changes = Vec::new();
        for tool in Tool::all() {
            let current_checksum = Self::compute_native_checksum(&tool);
            let mut state = read_active_state(&tool.id)?.unwrap_or_default();
            let last_checksum = state.native_checksum.clone();
            if last_checksum != current_checksum {
                // 标记脏，但保留旧 checksum 以便前端确认后再更新
                state.dirty = true;
                save_active_state(&tool.id, &state)?;

                changes.push(ExternalConfigChange {
                    tool_id: tool.id.clone(),
                    path: tool
                        .config_dir
                        .join(&tool.config_file)
                        .to_string_lossy()
                        .to_string(),
                    checksum: current_checksum.clone(),
                    detected_at: Utc::now(),
                    dirty: true,
                });
            } else if state.dirty {
                // 仍在脏状态时保持报告
                changes.push(ExternalConfigChange {
                    tool_id: tool.id.clone(),
                    path: tool
                        .config_dir
                        .join(&tool.config_file)
                        .to_string_lossy()
                        .to_string(),
                    checksum: current_checksum.clone(),
                    detected_at: Utc::now(),
                    dirty: true,
                });
            }
        }
        Ok(changes)
    }

    /// 直接标记外部修改（用于事件监听场景）。
    pub fn mark_external_change(
        tool: &Tool,
        path: std::path::PathBuf,
        checksum: Option<String>,
    ) -> Result<ExternalConfigChange> {
        let mut state = read_active_state(&tool.id)?.unwrap_or_default();
        // 若与当前记录的 checksum 一致，则视为内部写入，保持非脏状态
        let checksum_changed = state.native_checksum != checksum;
        state.dirty = checksum_changed;
        state.native_checksum = checksum.clone();
        save_active_state(&tool.id, &state)?;

        Ok(ExternalConfigChange {
            tool_id: tool.id.clone(),
            path: path.to_string_lossy().to_string(),
            checksum,
            detected_at: Utc::now(),
            dirty: state.dirty,
        })
    }

    /// 确认/清除外部修改状态，刷新 checksum。
    pub fn acknowledge_external_change(tool: &Tool) -> Result<()> {
        let current_checksum = Self::compute_native_checksum(tool);

        let mut state = read_active_state(&tool.id)?.unwrap_or_default();
        state.dirty = false;
        state.native_checksum = current_checksum;
        state.last_synced_at = Some(Utc::now());
        save_active_state(&tool.id, &state)?;
        Ok(())
    }
}
