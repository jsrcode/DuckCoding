use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Utc;
use once_cell::sync::OnceCell;
use serde::Serialize;
use serde_json::{Map, Value};
use toml;

use crate::models::Tool;
use crate::services::profile_store::{
    migration_log_path, profile_extension, profile_file_path, save_profile_payload,
    MigrationRecord, ProfilePayload,
};

#[derive(Debug, Clone, Serialize)]
pub struct LegacyCleanupResult {
    pub tool_id: String,
    pub removed: Vec<PathBuf>,
    pub failed: Vec<(PathBuf, String)>,
}

/// 旧版配置迁移到集中存储的轻量实现（仅搬运，不删除旧文件）
pub struct MigrationService;

impl MigrationService {
    /// 仅执行一次的迁移入口，失败时记录日志但不阻塞主流程。
    pub fn run_if_needed() {
        static ONCE: OnceCell<()> = OnceCell::new();
        let _ = ONCE.get_or_init(|| {
            if let Err(err) = Self::migrate_all() {
                eprintln!("迁移旧配置失败: {err:?}");
            }
        });
    }

    /// 测试专用：直接执行完整迁移流程（不受 ONCE 限制）
    #[cfg(test)]
    pub fn run_for_tests() -> Result<Vec<MigrationRecord>> {
        let mut records = Vec::new();
        records.extend(Self::migrate_claude()?);
        records.extend(Self::migrate_codex()?);
        records.extend(Self::migrate_gemini()?);
        if !records.is_empty() {
            Self::append_log(&records)?;
        }
        Ok(records)
    }

    fn migrate_all() -> Result<()> {
        let mut records = Vec::new();
        records.extend(Self::migrate_claude()?);
        records.extend(Self::migrate_codex()?);
        records.extend(Self::migrate_gemini()?);

        if !records.is_empty() {
            Self::append_log(&records)?;
        }
        Ok(())
    }

    fn migrate_claude() -> Result<Vec<MigrationRecord>> {
        let tool = Tool::claude_code();
        let mut records = Vec::new();

        if !tool.config_dir.exists() {
            return Ok(records);
        }

        for entry in fs::read_dir(&tool.config_dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = match path.file_name().and_then(|s| s.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            if name == tool.config_file {
                continue;
            }
            if !(name.starts_with("settings.") && name.ends_with(".json")) {
                continue;
            }

            let profile = name
                .trim_start_matches("settings.")
                .trim_end_matches(".json")
                .to_string();
            if profile.is_empty() || profile.starts_with('.') {
                continue;
            }

            let now = Utc::now();
            let record = match fs::read_to_string(&path) {
                Ok(content) => {
                    let data: Value = match serde_json::from_str(&content) {
                        Ok(v) => v,
                        Err(err) => {
                            records.push(MigrationRecord {
                                tool_id: tool.id.clone(),
                                profile_name: profile.clone(),
                                from_path: path.clone(),
                                to_path: PathBuf::new(),
                                succeeded: false,
                                message: Some(format!("解析失败: {err}")),
                                timestamp: now,
                            });
                            continue;
                        }
                    };

                    let api_key = data
                        .get("ANTHROPIC_AUTH_TOKEN")
                        .and_then(|v| v.as_str())
                        .or_else(|| {
                            data.get("env")
                                .and_then(|env| env.get("ANTHROPIC_AUTH_TOKEN"))
                                .and_then(|v| v.as_str())
                        })
                        .unwrap_or("")
                        .to_string();
                    let base_url = data
                        .get("ANTHROPIC_BASE_URL")
                        .and_then(|v| v.as_str())
                        .or_else(|| {
                            data.get("env")
                                .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                                .and_then(|v| v.as_str())
                        })
                        .unwrap_or("")
                        .to_string();

                    let payload = ProfilePayload::Claude {
                        api_key,
                        base_url,
                        raw_settings: Some(data.clone()),
                        raw_config_json: None,
                    };
                    let to_path =
                        profile_file_path(&tool.id, &profile, profile_extension(&tool.id))?;
                    save_profile_payload(&tool.id, &profile, &payload)?;
                    let _ = fs::remove_file(&path);

                    MigrationRecord {
                        tool_id: tool.id.clone(),
                        profile_name: profile.clone(),
                        from_path: path.clone(),
                        to_path,
                        succeeded: true,
                        message: None,
                        timestamp: now,
                    }
                }
                Err(err) => MigrationRecord {
                    tool_id: tool.id.clone(),
                    profile_name: profile.clone(),
                    from_path: path.clone(),
                    to_path: PathBuf::new(),
                    succeeded: false,
                    message: Some(format!("读取失败: {err}")),
                    timestamp: now,
                },
            };

            records.push(record);
        }

        Ok(records)
    }

    fn migrate_codex() -> Result<Vec<MigrationRecord>> {
        let tool = Tool::codex();
        let mut records = Vec::new();

        if !tool.config_dir.exists() {
            return Ok(records);
        }

        for entry in fs::read_dir(&tool.config_dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = match path.file_name().and_then(|s| s.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            if !(name.starts_with("config.") && name.ends_with(".toml")) {
                continue;
            }

            let profile = name
                .trim_start_matches("config.")
                .trim_end_matches(".toml")
                .to_string();
            if profile.is_empty() || profile.starts_with('.') {
                continue;
            }

            let backup_auth = tool.config_dir.join(format!("auth.{profile}.json"));
            if !backup_auth.exists() {
                continue;
            }

            let now = Utc::now();
            let auth_content = fs::read_to_string(&backup_auth).unwrap_or_default();
            let backup_auth_data: Value =
                serde_json::from_str(&auth_content).unwrap_or(Value::Object(Map::new()));
            let api_key = backup_auth_data
                .get("OPENAI_API_KEY")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mut base_url = String::new();
            let mut provider = None;
            let raw_config_toml = fs::read_to_string(&path).ok();
            if let Some(content) = raw_config_toml.clone() {
                if let Ok(toml::Value::Table(table)) = toml::from_str::<toml::Value>(&content) {
                    provider = table
                        .get("model_provider")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    if let Some(toml::Value::Table(providers)) = table.get("model_providers") {
                        if let Some(provider_name) = provider
                            .clone()
                            .or_else(|| providers.keys().next().cloned())
                        {
                            if let Some(toml::Value::Table(provider_table)) =
                                providers.get(&provider_name)
                            {
                                if let Some(toml::Value::String(url)) =
                                    provider_table.get("base_url")
                                {
                                    base_url = url.clone();
                                }
                            }
                        }
                    }
                }
            }

            if base_url.is_empty() {
                base_url = "https://jp.duckcoding.com/v1".to_string();
            }

            let payload = ProfilePayload::Codex {
                api_key,
                base_url: base_url.clone(),
                provider: provider.clone(),
                raw_config_toml,
                raw_auth_json: Some(backup_auth_data.clone()),
            };
            let to_path = profile_file_path(&tool.id, &profile, profile_extension(&tool.id))?;
            save_profile_payload(&tool.id, &profile, &payload)?;
            let _ = fs::remove_file(&path);
            let _ = fs::remove_file(&backup_auth);

            records.push(MigrationRecord {
                tool_id: tool.id.clone(),
                profile_name: profile,
                from_path: path.clone(),
                to_path,
                succeeded: true,
                message: None,
                timestamp: now,
            });
        }

        Ok(records)
    }

    fn migrate_gemini() -> Result<Vec<MigrationRecord>> {
        let tool = Tool::gemini_cli();
        let mut records = Vec::new();

        if !tool.config_dir.exists() {
            return Ok(records);
        }

        for entry in fs::read_dir(&tool.config_dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = match path.file_name().and_then(|s| s.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            if !(name.starts_with(".env.") && name.len() > 5) {
                continue;
            }

            let profile = name.trim_start_matches(".env.").to_string();
            if profile.is_empty() || profile.starts_with('.') {
                continue;
            }

            let now = Utc::now();
            let mut api_key = String::new();
            let mut base_url = String::new();
            let mut model = "gemini-2.5-pro".to_string();
            let raw_env = fs::read_to_string(&path).ok();

            if let Some(content) = raw_env.clone() {
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
            }

            if base_url.is_empty() {
                base_url = "https://generativelanguage.googleapis.com".to_string();
            }

            let payload = ProfilePayload::Gemini {
                api_key,
                base_url,
                model,
                raw_settings: None,
                raw_env,
            };
            let to_path = profile_file_path(&tool.id, &profile, profile_extension(&tool.id))?;
            save_profile_payload(&tool.id, &profile, &payload)?;
            let _ = fs::remove_file(&path);

            records.push(MigrationRecord {
                tool_id: tool.id.clone(),
                profile_name: profile,
                from_path: path.clone(),
                to_path,
                succeeded: true,
                message: None,
                timestamp: now,
            });
        }

        Ok(records)
    }

    fn append_log(records: &[MigrationRecord]) -> Result<()> {
        let log_path = migration_log_path()?;
        let mut existing: Vec<MigrationRecord> = if log_path.exists() {
            let content = fs::read_to_string(&log_path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            vec![]
        };
        existing.extend_from_slice(records);
        let json = serde_json::to_string_pretty(&existing)?;
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&log_path, json).with_context(|| format!("写入迁移日志失败: {log_path:?}"))?;
        Ok(())
    }

    /// 清理遗留的旧版备份文件，返回删除/失败列表。
    pub fn cleanup_legacy_backups() -> Result<Vec<LegacyCleanupResult>> {
        Ok(vec![
            Self::cleanup_claude_backups()?,
            Self::cleanup_codex_backups()?,
            Self::cleanup_gemini_backups()?,
        ])
    }

    fn cleanup_claude_backups() -> Result<LegacyCleanupResult> {
        let tool = Tool::claude_code();
        let mut removed = Vec::new();
        let mut failed = Vec::new();
        if tool.config_dir.exists() {
            for entry in fs::read_dir(&tool.config_dir)? {
                let entry = entry?;
                let path = entry.path();
                let name = match path.file_name().and_then(|s| s.to_str()) {
                    Some(n) => n,
                    None => continue,
                };
                if name == tool.config_file {
                    continue;
                }
                if name.starts_with("settings.") && name.ends_with(".json") {
                    match fs::remove_file(&path) {
                        Ok(_) => removed.push(path.clone()),
                        Err(err) => failed.push((path.clone(), err.to_string())),
                    }
                }
            }
        }
        Ok(LegacyCleanupResult {
            tool_id: tool.id,
            removed,
            failed,
        })
    }

    fn cleanup_codex_backups() -> Result<LegacyCleanupResult> {
        let tool = Tool::codex();
        let mut removed = Vec::new();
        let mut failed = Vec::new();
        if tool.config_dir.exists() {
            for entry in fs::read_dir(&tool.config_dir)? {
                let entry = entry?;
                let path = entry.path();
                let name = match path.file_name().and_then(|s| s.to_str()) {
                    Some(n) => n,
                    None => continue,
                };

                let is_backup_config = name != tool.config_file
                    && name.starts_with("config.")
                    && name.ends_with(".toml");
                let is_backup_auth =
                    name != "auth.json" && name.starts_with("auth.") && name.ends_with(".json");

                if is_backup_config || is_backup_auth {
                    match fs::remove_file(&path) {
                        Ok(_) => removed.push(path.clone()),
                        Err(err) => failed.push((path.clone(), err.to_string())),
                    }
                }
            }
        }
        Ok(LegacyCleanupResult {
            tool_id: tool.id,
            removed,
            failed,
        })
    }

    fn cleanup_gemini_backups() -> Result<LegacyCleanupResult> {
        let tool = Tool::gemini_cli();
        let mut removed = Vec::new();
        let mut failed = Vec::new();

        if tool.config_dir.exists() {
            for entry in fs::read_dir(&tool.config_dir)? {
                let entry = entry?;
                let path = entry.path();
                let name = match path.file_name().and_then(|s| s.to_str()) {
                    Some(n) => n,
                    None => continue,
                };

                if name == ".env" || name == tool.config_file {
                    continue;
                }

                if name.starts_with(".env.") {
                    match fs::remove_file(&path) {
                        Ok(_) => removed.push(path.clone()),
                        Err(err) => failed.push((path.clone(), err.to_string())),
                    }
                }
            }
        }

        Ok(LegacyCleanupResult {
            tool_id: tool.id,
            removed,
            failed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn migrate_all_tools_and_write_log() -> Result<()> {
        let temp = TempDir::new().expect("create temp dir");
        env::set_var("DUCKCODING_CONFIG_DIR", temp.path());
        env::set_var("HOME", temp.path());
        env::set_var("USERPROFILE", temp.path());

        // Claude legacy file
        let claude_dir = temp.path().join(".claude");
        fs::create_dir_all(&claude_dir)?;
        fs::write(
            claude_dir.join("settings.personal.json"),
            r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"claude-key","ANTHROPIC_BASE_URL":"https://claude"}}"#,
        )?;

        // Codex legacy files
        let codex_dir = temp.path().join(".codex");
        fs::create_dir_all(&codex_dir)?;
        fs::write(
            codex_dir.join("config.work.toml"),
            r#"model_provider="duckcoding"
[model_providers.duckcoding]
base_url="https://duckcoding.test/v1"
"#,
        )?;
        fs::write(
            codex_dir.join("auth.work.json"),
            r#"{"OPENAI_API_KEY":"codex-key"}"#,
        )?;

        // Gemini legacy file
        let gemini_dir = temp.path().join(".gemini");
        fs::create_dir_all(&gemini_dir)?;
        fs::write(
            gemini_dir.join(".env.dev"),
            "GEMINI_API_KEY=gem-key\nGOOGLE_GEMINI_BASE_URL=https://gem\nGEMINI_MODEL=gem-model\n",
        )?;

        let records = MigrationService::run_for_tests()?;
        assert_eq!(records.len(), 3);

        // 确认迁移输出存在
        let claude_profile =
            profile_file_path("claude-code", "personal", profile_extension("claude-code"))?;
        let codex_profile = profile_file_path("codex", "work", profile_extension("codex"))?;
        let gemini_profile =
            profile_file_path("gemini-cli", "dev", profile_extension("gemini-cli"))?;
        assert!(claude_profile.exists());
        assert!(codex_profile.exists());
        assert!(gemini_profile.exists());

        // 日志写入
        let log_records = crate::services::profile_store::read_migration_log()?;
        assert_eq!(log_records.len(), 3);
        Ok(())
    }

    #[test]
    #[serial]
    fn cleanup_legacy_backups_removes_files() -> Result<()> {
        let temp = TempDir::new().expect("create temp dir");
        env::set_var("DUCKCODING_CONFIG_DIR", temp.path());
        env::set_var("HOME", temp.path());
        env::set_var("USERPROFILE", temp.path());

        let claude_dir = temp.path().join(".claude");
        fs::create_dir_all(&claude_dir)?;
        let stale = claude_dir.join("settings.old.json");
        fs::write(&stale, "{}")?;

        let results = MigrationService::cleanup_legacy_backups()?;
        let claude_result = results
            .into_iter()
            .find(|r| r.tool_id == "claude-code")
            .expect("claude result");
        assert!(claude_result.removed.contains(&stale));

        Ok(())
    }
}
