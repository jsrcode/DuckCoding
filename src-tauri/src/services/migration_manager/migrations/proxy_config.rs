// Proxy 配置重构迁移
//
// 将旧的 transparent_proxy_* 字段迁移到 proxy_configs["claude-code"]

use crate::data::DataManager;
use crate::models::GlobalConfig;
use crate::services::migration_manager::migration_trait::{Migration, MigrationResult};
use crate::utils::config::global_config_path;
use anyhow::Result;
use async_trait::async_trait;

/// Proxy 配置重构迁移（目标版本 1.3.9）
pub struct ProxyConfigMigration;

impl Default for ProxyConfigMigration {
    fn default() -> Self {
        Self::new()
    }
}

impl ProxyConfigMigration {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Migration for ProxyConfigMigration {
    fn id(&self) -> &str {
        "proxy_config_refactor_v1"
    }

    fn name(&self) -> &str {
        "Proxy 配置重构迁移"
    }

    fn target_version(&self) -> &str {
        "1.4.0"
    }

    async fn execute(&self) -> Result<MigrationResult> {
        tracing::info!("开始执行 Proxy 配置重构迁移");

        // 读取配置
        let config_path = global_config_path().map_err(|e| anyhow::anyhow!(e))?;
        let manager = DataManager::new();

        let config_value = manager.json_uncached().read(&config_path)?;
        let mut config: GlobalConfig = serde_json::from_value(config_value)?;

        let mut migrated = false;

        // 检查是否需要迁移
        if config.transparent_proxy_enabled
            || config.transparent_proxy_api_key.is_some()
            || config.transparent_proxy_real_api_key.is_some()
        {
            // 获取或创建 claude-code 的配置
            let claude_config = config
                .proxy_configs
                .entry("claude-code".to_string())
                .or_default();

            // 只有当新配置还是默认值时才迁移
            if !claude_config.enabled && claude_config.real_api_key.is_none() {
                claude_config.enabled = config.transparent_proxy_enabled;
                claude_config.port = config.transparent_proxy_port;
                claude_config.local_api_key = config.transparent_proxy_api_key.clone();
                claude_config.real_api_key = config.transparent_proxy_real_api_key.clone();
                claude_config.real_base_url = config.transparent_proxy_real_base_url.clone();
                claude_config.allow_public = config.transparent_proxy_allow_public;

                migrated = true;
            }

            // 清除旧字段
            config.transparent_proxy_enabled = false;
            config.transparent_proxy_api_key = None;
            config.transparent_proxy_real_api_key = None;
            config.transparent_proxy_real_base_url = None;

            // 保存配置
            let config_value = serde_json::to_value(&config)?;
            manager.json_uncached().write(&config_path, &config_value)?;
        }

        let message = if migrated {
            "成功迁移 Proxy 配置到新架构"
        } else {
            "无需迁移（已迁移或无旧配置）"
        };

        Ok(MigrationResult {
            migration_id: self.id().to_string(),
            success: true,
            message: message.to_string(),
            records_migrated: if migrated { 1 } else { 0 },
            duration_secs: 0.0,
        })
    }
}
