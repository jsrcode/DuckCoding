//! 迁移测试用例
//!
//! 模拟现有配置管理代码迁移到 DataManager 的场景，确保新 API 的正确性。
//! 所有测试使用 tempfile::TempDir 隔离，不修改真实文件。

use super::manager::DataManager;
use super::Result;
use serde_json::json;
use std::collections::HashMap;
use tempfile::TempDir;
use toml_edit::DocumentMut;

#[cfg(test)]
mod utils_config_migration {
    use super::*;

    /// 模拟 utils/config.rs 的 read_global_config 函数
    ///
    /// 旧实现：直接使用 fs::read_to_string + serde_json::from_str
    /// 新实现：使用 DataManager.json().read()
    #[test]
    fn test_migrate_read_global_config() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // 模拟旧的全局配置结构
        let old_config = json!({
            "transparent_proxy_enabled": true,
            "transparent_proxy_port": 8787,
            "transparent_proxy_api_key": "local-key",
            "transparent_proxy_real_api_key": "sk-real-key",
            "proxy_configs": {
                "claude-code": {
                    "enabled": false,
                    "port": 8787
                }
            }
        });

        // 使用 DataManager 写入配置
        let manager = DataManager::new();
        manager.json().write(&config_path, &old_config)?;

        // 验证：使用 DataManager 读取配置，结果应与旧方式一致
        let loaded_config = manager.json().read(&config_path)?;

        assert_eq!(
            loaded_config["transparent_proxy_enabled"],
            json!(true),
            "透明代理启用状态应该匹配"
        );
        assert_eq!(
            loaded_config["transparent_proxy_port"],
            json!(8787),
            "透明代理端口应该匹配"
        );
        assert_eq!(
            loaded_config["transparent_proxy_api_key"],
            json!("local-key"),
            "本地 API Key 应该匹配"
        );

        // 验证嵌套配置
        assert_eq!(
            loaded_config["proxy_configs"]["claude-code"]["enabled"],
            json!(false),
            "Claude Code 代理启用状态应该匹配"
        );

        Ok(())
    }

    /// 模拟 utils/config.rs 的 write_global_config 函数
    ///
    /// 旧实现：serde_json::to_string_pretty + fs::write
    /// 新实现：使用 DataManager.json().write()
    #[test]
    fn test_migrate_write_global_config() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let manager = DataManager::new();

        // 模拟新配置写入
        let new_config = json!({
            "proxy_configs": {
                "claude-code": {
                    "enabled": true,
                    "port": 8787,
                    "local_api_key": "new-local-key",
                    "real_api_key": "sk-new-real-key"
                }
            },
            "session_endpoint_config_enabled": false
        });

        manager.json().write(&config_path, &new_config)?;

        // 验证：读取配置，确保写入成功
        let loaded = manager.json().read(&config_path)?;

        assert_eq!(
            loaded["proxy_configs"]["claude-code"]["enabled"],
            json!(true),
            "配置写入后应该可以正确读取"
        );
        assert_eq!(
            loaded["proxy_configs"]["claude-code"]["local_api_key"],
            json!("new-local-key"),
            "API Key 写入后应该可以正确读取"
        );

        Ok(())
    }

    /// 测试配置文件缓存机制
    ///
    /// 旧实现：每次读取都访问文件系统
    /// 新实现：使用 DataManager.json() 启用缓存，验证缓存命中
    #[test]
    fn test_migrate_config_caching() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let manager = DataManager::new();

        let config = json!({
            "test_field": "value1"
        });

        manager.json().write(&config_path, &config)?;

        // 第一次读取（缓存未命中）
        let read1 = manager.json().read(&config_path)?;
        assert_eq!(read1["test_field"], json!("value1"));

        // 第二次读取（缓存命中）
        let read2 = manager.json().read(&config_path)?;
        assert_eq!(read2["test_field"], json!("value1"));

        // 验证：缓存命中时，读取结果应该一致
        assert_eq!(read1, read2, "缓存读取结果应该一致");

        Ok(())
    }
}

#[cfg(test)]
mod services_config_migration {
    use super::*;

    /// 模拟 services/config.rs 的 read_claude_settings 函数
    ///
    /// 旧实现：fs::read_to_string + serde_json::from_str
    /// 新实现：使用 DataManager.json_uncached().read()
    #[test]
    fn test_migrate_read_claude_settings() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let settings_path = temp_dir.path().join("settings.json");

        let manager = DataManager::new();

        // 模拟 Claude Code settings.json 结构
        let settings = json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "sk-ant-test-key",
                "ANTHROPIC_BASE_URL": "https://api.anthropic.com"
            },
            "ide": {
                "enabled": true
            }
        });

        manager.json_uncached().write(&settings_path, &settings)?;

        // 验证：读取配置，确保结构正确
        let loaded = manager.json_uncached().read(&settings_path)?;

        assert_eq!(
            loaded["env"]["ANTHROPIC_AUTH_TOKEN"],
            json!("sk-ant-test-key"),
            "API Token 应该匹配"
        );
        assert_eq!(
            loaded["env"]["ANTHROPIC_BASE_URL"],
            json!("https://api.anthropic.com"),
            "Base URL 应该匹配"
        );

        Ok(())
    }

    /// 模拟 services/config.rs 的 save_claude_settings 函数
    ///
    /// 旧实现：serde_json::to_string_pretty + fs::write
    /// 新实现：使用 DataManager.json_uncached().write()
    #[test]
    fn test_migrate_save_claude_settings() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let settings_path = temp_dir.path().join("settings.json");

        let manager = DataManager::new();

        // 模拟保存新的 Claude 配置
        let new_settings = json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "sk-ant-new-key",
                "ANTHROPIC_BASE_URL": "https://custom.api.com"
            }
        });

        manager
            .json_uncached()
            .write(&settings_path, &new_settings)?;

        // 验证：读取配置，确保保存成功
        let loaded = manager.json_uncached().read(&settings_path)?;

        assert_eq!(
            loaded["env"]["ANTHROPIC_AUTH_TOKEN"],
            json!("sk-ant-new-key"),
            "新 API Key 应该保存成功"
        );

        Ok(())
    }

    /// 模拟 services/config.rs 的 read_codex_settings 函数
    ///
    /// 旧实现：fs::read_to_string + toml::from_str
    /// 新实现：使用 DataManager.toml().read()
    #[test]
    fn test_migrate_read_codex_config() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let manager = DataManager::new();

        // 模拟 Codex config.toml 结构（使用 toml_edit::DocumentMut）
        let config_str = r#"
model = "gpt-5-codex"
model_provider = "duckcoding"

[model_providers.duckcoding]
name = "duckcoding"
base_url = "https://jp.duckcoding.com/v1"
wire_api = "responses"
requires_openai_auth = true
"#;

        let doc: DocumentMut = config_str.parse().unwrap();

        manager.toml().write(&config_path, &doc)?;

        // 验证：读取配置，确保结构正确
        let loaded = manager.toml().read(&config_path)?;

        assert_eq!(
            loaded["model"].as_str(),
            Some("gpt-5-codex"),
            "model 字段应该匹配"
        );
        assert_eq!(
            loaded["model_provider"].as_str(),
            Some("duckcoding"),
            "model_provider 应该匹配"
        );
        assert_eq!(
            loaded["model_providers"]["duckcoding"]["base_url"].as_str(),
            Some("https://jp.duckcoding.com/v1"),
            "base_url 应该匹配"
        );

        Ok(())
    }

    /// 模拟 services/config.rs 的 save_codex_settings 函数
    ///
    /// 旧实现：toml_edit 保留注释 + fs::write
    /// 新实现：使用 DataManager.toml().write()（保留注释的能力通过 toml_edit 内部实现）
    #[test]
    fn test_migrate_save_codex_config() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let manager = DataManager::new();

        // 模拟现有配置
        let existing_config_str = r#"
# Codex 配置文件
model = "gpt-5-codex"
model_provider = "duckcoding"

[model_providers.duckcoding]
base_url = "https://old.duckcoding.com/v1"
"#;

        let existing_doc: DocumentMut = existing_config_str.parse().unwrap();
        manager.toml().write(&config_path, &existing_doc)?;

        // 模拟更新配置
        let updated_config_str = r#"
model = "gpt-5-codex"
model_provider = "duckcoding"

[model_providers.duckcoding]
base_url = "https://new.duckcoding.com/v1"
"#;

        let updated_doc: DocumentMut = updated_config_str.parse().unwrap();
        manager.toml().write(&config_path, &updated_doc)?;

        // 验证：读取配置，确保更新成功
        let loaded = manager.toml().read(&config_path)?;

        assert_eq!(
            loaded["model_providers"]["duckcoding"]["base_url"].as_str(),
            Some("https://new.duckcoding.com/v1"),
            "base_url 应该更新成功"
        );

        Ok(())
    }

    /// 模拟 services/config.rs 的 read_gemini_settings 函数
    ///
    /// 旧实现：fs::read_to_string 读取 .env
    /// 新实现：使用 DataManager.env().read()
    #[test]
    fn test_migrate_read_gemini_env() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let env_path = temp_dir.path().join(".env");

        let manager = DataManager::new();

        // 模拟 .env 文件内容
        let mut env_vars = HashMap::new();
        env_vars.insert("GEMINI_API_KEY".to_string(), "test-gemini-key".to_string());
        env_vars.insert(
            "GOOGLE_GEMINI_BASE_URL".to_string(),
            "https://generativelanguage.googleapis.com".to_string(),
        );
        env_vars.insert("GEMINI_MODEL".to_string(), "gemini-2.5-pro".to_string());

        manager.env().write(&env_path, &env_vars)?;

        // 验证：读取 .env，确保内容正确
        let loaded = manager.env().read(&env_path)?;

        assert_eq!(
            loaded.get("GEMINI_API_KEY"),
            Some(&"test-gemini-key".to_string()),
            "GEMINI_API_KEY 应该匹配"
        );
        assert_eq!(
            loaded.get("GOOGLE_GEMINI_BASE_URL"),
            Some(&"https://generativelanguage.googleapis.com".to_string()),
            "GOOGLE_GEMINI_BASE_URL 应该匹配"
        );
        assert_eq!(
            loaded.get("GEMINI_MODEL"),
            Some(&"gemini-2.5-pro".to_string()),
            "GEMINI_MODEL 应该匹配"
        );

        Ok(())
    }

    /// 模拟 services/config.rs 的 save_gemini_settings 函数
    ///
    /// 旧实现：手动拼接 key=value 格式 + fs::write
    /// 新实现：使用 DataManager.env().write()
    #[test]
    fn test_migrate_save_gemini_env() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let env_path = temp_dir.path().join(".env");

        let manager = DataManager::new();

        // 模拟保存新的环境变量
        let mut new_env_vars = HashMap::new();
        new_env_vars.insert("GEMINI_API_KEY".to_string(), "new-gemini-key".to_string());
        new_env_vars.insert(
            "GOOGLE_GEMINI_BASE_URL".to_string(),
            "https://custom.gemini.com".to_string(),
        );

        manager.env().write(&env_path, &new_env_vars)?;

        // 验证：读取 .env，确保保存成功
        let loaded = manager.env().read(&env_path)?;

        assert_eq!(
            loaded.get("GEMINI_API_KEY"),
            Some(&"new-gemini-key".to_string()),
            "新 API Key 应该保存成功"
        );
        assert_eq!(
            loaded.get("GOOGLE_GEMINI_BASE_URL"),
            Some(&"https://custom.gemini.com".to_string()),
            "新 Base URL 应该保存成功"
        );

        Ok(())
    }

    /// 测试工具配置的实时更新（无缓存模式）
    ///
    /// 旧实现：每次读取都直接访问文件
    /// 新实现：使用 DataManager.json_uncached() 确保实时读取
    #[test]
    fn test_migrate_uncached_tool_config() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let settings_path = temp_dir.path().join("settings.json");

        let manager = DataManager::new();

        // 写入初始配置
        let initial_config = json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "key-v1"
            }
        });

        manager
            .json_uncached()
            .write(&settings_path, &initial_config)?;

        // 读取第一次
        let read1 = manager.json_uncached().read(&settings_path)?;
        assert_eq!(read1["env"]["ANTHROPIC_AUTH_TOKEN"], json!("key-v1"));

        // 模拟外部修改（直接写入新配置）
        let updated_config = json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "key-v2"
            }
        });

        manager
            .json_uncached()
            .write(&settings_path, &updated_config)?;

        // 读取第二次（无缓存模式应该立即反映修改）
        let read2 = manager.json_uncached().read(&settings_path)?;
        assert_eq!(
            read2["env"]["ANTHROPIC_AUTH_TOKEN"],
            json!("key-v2"),
            "无缓存模式应该立即读取到最新配置"
        );

        Ok(())
    }
}

#[cfg(test)]
mod profile_store_migration {
    use super::*;

    /// 模拟 services/profile_store.rs 的 save_profile_payload 函数
    ///
    /// 旧实现：手动序列化 JSON + fs::write
    /// 新实现：使用 DataManager.json().write()
    #[test]
    fn test_migrate_save_profile() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profiles").join("claude-code");
        std::fs::create_dir_all(&profile_path).unwrap();

        let manager = DataManager::new();

        // 模拟 Profile 数据结构
        let profile_payload = json!({
            "tool_id": "claude-code",
            "api_key": "sk-profile-key",
            "base_url": "https://api.anthropic.com",
            "raw_settings": {
                "env": {
                    "ANTHROPIC_AUTH_TOKEN": "sk-profile-key",
                    "ANTHROPIC_BASE_URL": "https://api.anthropic.com"
                }
            }
        });

        let profile_file = profile_path.join("default.json");
        manager.json().write(&profile_file, &profile_payload)?;

        // 验证：读取 Profile，确保保存成功
        let loaded = manager.json().read(&profile_file)?;

        assert_eq!(
            loaded["api_key"],
            json!("sk-profile-key"),
            "Profile API Key 应该保存成功"
        );
        assert_eq!(
            loaded["base_url"],
            json!("https://api.anthropic.com"),
            "Profile Base URL 应该保存成功"
        );

        Ok(())
    }

    /// 模拟 services/profile_store.rs 的 load_profile_payload 函数
    ///
    /// 旧实现：fs::read_to_string + serde_json::from_str
    /// 新实现：使用 DataManager.json().read()
    #[test]
    fn test_migrate_load_profile() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profiles").join("codex");
        std::fs::create_dir_all(&profile_path).unwrap();

        let manager = DataManager::new();

        // 模拟 Codex Profile
        let profile_payload = json!({
            "tool_id": "codex",
            "api_key": "codex-key",
            "base_url": "https://jp.duckcoding.com/v1",
            "provider": "duckcoding"
        });

        let profile_file = profile_path.join("production.json");
        manager.json().write(&profile_file, &profile_payload)?;

        // 验证：读取 Profile，确保加载成功
        let loaded = manager.json().read(&profile_file)?;

        assert_eq!(
            loaded["tool_id"],
            json!("codex"),
            "Profile tool_id 应该加载成功"
        );
        assert_eq!(
            loaded["provider"],
            json!("duckcoding"),
            "Profile provider 应该加载成功"
        );

        Ok(())
    }

    /// 模拟 services/profile_store.rs 的批量读取 Profile
    ///
    /// 旧实现：fs::read_dir + 遍历读取
    /// 新实现：使用 DataManager.json().read() 批量读取
    #[test]
    fn test_migrate_list_profiles() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profiles").join("gemini-cli");
        std::fs::create_dir_all(&profile_path).unwrap();

        let manager = DataManager::new();

        // 模拟创建多个 Profile
        let profiles = vec![
            ("dev", "dev-key", "https://dev.gemini.com"),
            ("staging", "staging-key", "https://staging.gemini.com"),
            ("production", "prod-key", "https://gemini.com"),
        ];

        for (name, api_key, base_url) in &profiles {
            let profile_data = json!({
                "tool_id": "gemini-cli",
                "api_key": api_key,
                "base_url": base_url,
                "model": "gemini-2.5-pro"
            });

            let profile_file = profile_path.join(format!("{}.json", name));
            manager.json().write(&profile_file, &profile_data)?;
        }

        // 验证：批量读取所有 Profile
        let mut loaded_profiles = Vec::new();
        for entry in std::fs::read_dir(&profile_path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let profile = manager.json().read(&path)?;
                loaded_profiles.push(profile);
            }
        }

        assert_eq!(loaded_profiles.len(), 3, "应该读取到 3 个 Profile");

        // 验证：所有 Profile 的 tool_id 应该是 gemini-cli
        for profile in &loaded_profiles {
            assert_eq!(
                profile["tool_id"],
                json!("gemini-cli"),
                "所有 Profile 的 tool_id 应该一致"
            );
        }

        Ok(())
    }

    /// 测试 Profile 缓存机制
    ///
    /// 旧实现：频繁读取 Profile 文件
    /// 新实现：使用 DataManager.json() 启用缓存，减少文件 I/O
    #[test]
    fn test_migrate_profile_caching() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profiles").join("claude-code");
        std::fs::create_dir_all(&profile_path).unwrap();

        let manager = DataManager::new();

        // 创建大量 Profile（模拟批量读取场景）
        for i in 0..10 {
            let profile_data = json!({
                "tool_id": "claude-code",
                "api_key": format!("key-{}", i),
                "base_url": "https://api.anthropic.com"
            });

            let profile_file = profile_path.join(format!("profile-{}.json", i));
            manager.json().write(&profile_file, &profile_data)?;
        }

        // 第一次批量读取（缓存未命中）
        let mut first_read = Vec::new();
        for i in 0..10 {
            let profile_file = profile_path.join(format!("profile-{}.json", i));
            let profile = manager.json().read(&profile_file)?;
            first_read.push(profile);
        }

        // 第二次批量读取（缓存命中）
        let mut second_read = Vec::new();
        for i in 0..10 {
            let profile_file = profile_path.join(format!("profile-{}.json", i));
            let profile = manager.json().read(&profile_file)?;
            second_read.push(profile);
        }

        // 验证：两次读取结果应该一致
        for i in 0..10 {
            assert_eq!(
                first_read[i], second_read[i],
                "缓存读取结果应该与首次读取一致"
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// 综合测试：模拟完整的配置迁移流程
    ///
    /// 场景：从旧的配置管理迁移到 DataManager
    #[test]
    fn test_full_migration_workflow() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let manager = DataManager::new();

        // 步骤 1：迁移全局配置
        let global_config_path = temp_dir.path().join("config.json");
        let global_config = json!({
            "proxy_configs": {
                "claude-code": {
                    "enabled": true,
                    "port": 8787
                }
            }
        });
        manager.json().write(&global_config_path, &global_config)?;

        // 步骤 2：迁移 Claude Code 工具配置
        let claude_settings_path = temp_dir.path().join("claude").join("settings.json");
        std::fs::create_dir_all(claude_settings_path.parent().unwrap()).unwrap();

        let claude_settings = json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "sk-claude-key",
                "ANTHROPIC_BASE_URL": "https://api.anthropic.com"
            }
        });
        manager
            .json_uncached()
            .write(&claude_settings_path, &claude_settings)?;

        // 步骤 3：迁移 Codex 工具配置
        let codex_config_path = temp_dir.path().join("codex").join("config.toml");
        std::fs::create_dir_all(codex_config_path.parent().unwrap()).unwrap();

        let codex_config_str = r#"
model = "gpt-5-codex"
model_provider = "duckcoding"

[model_providers.duckcoding]
base_url = "https://jp.duckcoding.com/v1"
"#;
        let codex_config: DocumentMut = codex_config_str.parse().unwrap();
        manager.toml().write(&codex_config_path, &codex_config)?;

        // 步骤 4：迁移 Gemini CLI 工具配置
        let gemini_env_path = temp_dir.path().join("gemini").join(".env");
        std::fs::create_dir_all(gemini_env_path.parent().unwrap()).unwrap();

        let mut gemini_env = HashMap::new();
        gemini_env.insert("GEMINI_API_KEY".to_string(), "gemini-key".to_string());
        gemini_env.insert(
            "GOOGLE_GEMINI_BASE_URL".to_string(),
            "https://generativelanguage.googleapis.com".to_string(),
        );
        manager.env().write(&gemini_env_path, &gemini_env)?;

        // 步骤 5：迁移 Profile 数据
        let profile_dir = temp_dir.path().join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        for tool_id in &["claude-code", "codex", "gemini-cli"] {
            let tool_profile_dir = profile_dir.join(tool_id);
            std::fs::create_dir_all(&tool_profile_dir).unwrap();

            let profile_data = json!({
                "tool_id": tool_id,
                "api_key": format!("{}-profile-key", tool_id),
                "base_url": "https://example.com"
            });

            let profile_file = tool_profile_dir.join("default.json");
            manager.json().write(&profile_file, &profile_data)?;
        }

        // 验证：确保所有迁移数据可以正确读取
        let loaded_global = manager.json().read(&global_config_path)?;
        assert!(
            loaded_global["proxy_configs"]["claude-code"]["enabled"]
                .as_bool()
                .unwrap(),
            "全局配置应该迁移成功"
        );

        let loaded_claude = manager.json_uncached().read(&claude_settings_path)?;
        assert_eq!(
            loaded_claude["env"]["ANTHROPIC_AUTH_TOKEN"],
            json!("sk-claude-key"),
            "Claude Code 配置应该迁移成功"
        );

        let loaded_codex = manager.toml().read(&codex_config_path)?;
        assert_eq!(
            loaded_codex["model"].as_str(),
            Some("gpt-5-codex"),
            "Codex 配置应该迁移成功"
        );

        let loaded_gemini = manager.env().read(&gemini_env_path)?;
        assert_eq!(
            loaded_gemini.get("GEMINI_API_KEY"),
            Some(&"gemini-key".to_string()),
            "Gemini CLI 配置应该迁移成功"
        );

        // 验证：Profile 数据迁移成功
        for tool_id in &["claude-code", "codex", "gemini-cli"] {
            let profile_file = profile_dir.join(tool_id).join("default.json");
            let profile = manager.json().read(&profile_file)?;
            assert_eq!(
                profile["tool_id"],
                json!(tool_id),
                "{} 的 Profile 应该迁移成功",
                tool_id
            );
        }

        Ok(())
    }

    /// 测试缓存失效机制
    ///
    /// 场景：配置文件被外部修改后，缓存应该自动失效
    #[test]
    fn test_cache_invalidation_on_file_change() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let manager = DataManager::new();

        // 写入初始配置
        let initial_config = json!({
            "version": 1
        });
        manager.json().write(&config_path, &initial_config)?;

        // 第一次读取（缓存）
        let read1 = manager.json().read(&config_path)?;
        assert_eq!(read1["version"], json!(1));

        // 模拟外部修改（修改文件内容）
        let updated_config = json!({
            "version": 2
        });
        manager.json().write(&config_path, &updated_config)?;

        // 第二次读取（缓存应该失效）
        let read2 = manager.json().read(&config_path)?;
        assert_eq!(read2["version"], json!(2), "缓存应该在文件修改后失效");

        Ok(())
    }
}
