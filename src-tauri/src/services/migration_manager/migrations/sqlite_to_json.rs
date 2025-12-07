// SQLite → JSON 迁移
//
// 将 tool_instances.db 迁移到 tools.json

use crate::services::migration_manager::migration_trait::{Migration, MigrationResult};
use crate::services::tool::ToolInstanceDB;
use anyhow::Result;
use async_trait::async_trait;

/// SQLite → JSON 迁移（目标版本 1.4.0）
pub struct SqliteToJsonMigration;

impl Default for SqliteToJsonMigration {
    fn default() -> Self {
        Self::new()
    }
}

impl SqliteToJsonMigration {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Migration for SqliteToJsonMigration {
    fn id(&self) -> &str {
        "sqlite_to_json_v1"
    }

    fn name(&self) -> &str {
        "SQLite → JSON 迁移"
    }

    fn target_version(&self) -> &str {
        "1.4.0"
    }

    async fn execute(&self) -> Result<MigrationResult> {
        tracing::info!("开始执行 SQLite → JSON 迁移");

        // 调用 ToolInstanceDB 的迁移方法
        let db = ToolInstanceDB::new()?;
        db.migrate_from_sqlite()?;

        // 统计迁移的记录数
        let instances = db.get_all_instances()?;
        let count = instances.len();

        Ok(MigrationResult {
            migration_id: self.id().to_string(),
            success: true,
            message: format!("成功迁移 {} 个工具实例到 tools.json", count),
            records_migrated: count,
            duration_secs: 0.0, // 由 MigrationManager 填充
        })
    }

    async fn rollback(&self) -> Result<()> {
        // SQLite → JSON 迁移支持回滚
        tracing::warn!("回滚迁移：恢复 tool_instances.db");

        let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("无法获取用户主目录"))?;
        let duckcoding_dir = home_dir.join(".duckcoding");
        let backup_path = duckcoding_dir.join("tool_instances.db.backup");
        let db_path = duckcoding_dir.join("tool_instances.db");

        if backup_path.exists() {
            std::fs::rename(&backup_path, &db_path)?;
            tracing::info!("已恢复 tool_instances.db");
        } else {
            anyhow::bail!("备份文件不存在，无法回滚");
        }

        Ok(())
    }
}
