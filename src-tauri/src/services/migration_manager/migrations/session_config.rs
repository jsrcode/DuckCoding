// Session 配置拆分迁移
//
// 将全局 session_endpoint_config_enabled 迁移到工具级

use crate::data::DataManager;
use crate::models::GlobalConfig;
use crate::services::migration_manager::migration_trait::{Migration, MigrationResult};
use crate::utils::config::global_config_path;
use anyhow::Result;
use async_trait::async_trait;

/// Session 配置拆分迁移（目标版本 1.4.0）
pub struct SessionConfigMigration;

impl Default for SessionConfigMigration {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionConfigMigration {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Migration for SessionConfigMigration {
    fn id(&self) -> &str {
        "session_config_split_v1"
    }

    fn name(&self) -> &str {
        "Session 配置拆分迁移"
    }

    fn target_version(&self) -> &str {
        "1.4.0"
    }

    async fn execute(&self) -> Result<MigrationResult> {
        tracing::info!("开始执行 Session 配置拆分迁移");

        // 读取配置
        let config_path = global_config_path().map_err(|e| anyhow::anyhow!(e))?;
        let manager = DataManager::new();

        let config_value = manager.json_uncached().read(&config_path)?;
        let mut config: GlobalConfig = serde_json::from_value(config_value)?;

        let mut migrated_count = 0;

        // 仅在全局开关为 true 时进行迁移
        if config.session_endpoint_config_enabled {
            for tool_config in config.proxy_configs.values_mut() {
                // 仅迁移尚未设置的工具
                if !tool_config.session_endpoint_config_enabled {
                    tool_config.session_endpoint_config_enabled = true;
                    migrated_count += 1;
                }
            }

            // 清除全局标志
            config.session_endpoint_config_enabled = false;

            // 保存配置
            let config_value = serde_json::to_value(&config)?;
            manager.json_uncached().write(&config_path, &config_value)?;
        }

        let message = if migrated_count > 0 {
            format!("成功迁移 {} 个工具的 Session 配置", migrated_count)
        } else {
            "无需迁移（已迁移或全局开关未启用）".to_string()
        };

        Ok(MigrationResult {
            migration_id: self.id().to_string(),
            success: true,
            message,
            records_migrated: migrated_count,
            duration_secs: 0.0,
        })
    }
}
