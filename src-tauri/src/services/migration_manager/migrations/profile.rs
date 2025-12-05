// Profile 配置迁移
//
// 将旧的 settings.{profile}.json 迁移到 Profile 系统

use crate::services::migration::MigrationService;
use crate::services::migration_manager::migration_trait::{Migration, MigrationResult};
use anyhow::Result;
use async_trait::async_trait;

/// Profile 配置迁移（目标版本 1.3.8）
pub struct ProfileMigration;

impl ProfileMigration {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Migration for ProfileMigration {
    fn id(&self) -> &str {
        "profile_migration_v1"
    }

    fn name(&self) -> &str {
        "Profile 配置迁移"
    }

    fn target_version(&self) -> &str {
        "1.3.8"
    }

    async fn execute(&self) -> Result<MigrationResult> {
        tracing::info!("开始执行 Profile 配置迁移");

        // 调用旧的 MigrationService 逻辑
        MigrationService::run_if_needed();

        // 读取迁移日志统计
        let log = crate::services::profile_store::read_migration_log().unwrap_or_default();
        let count = log.len();

        Ok(MigrationResult {
            migration_id: self.id().to_string(),
            success: true,
            message: format!("Profile 迁移完成（共 {} 条记录）", count),
            records_migrated: count,
            duration_secs: 0.0,
        })
    }
}
