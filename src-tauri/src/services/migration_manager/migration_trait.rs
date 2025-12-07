// Migration Manager - 统一迁移管理器
//
// 基于版本号驱动的数据迁移系统

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// 迁移接口
#[async_trait]
pub trait Migration: Send + Sync {
    /// 迁移唯一标识（如 "sqlite_to_json"）
    fn id(&self) -> &str;

    /// 迁移名称（用于日志）
    fn name(&self) -> &str;

    /// 目标版本号（迁移执行后达到的版本）
    ///
    /// 示例：
    /// - "1.3.9" - SQLite → JSON 迁移
    /// - "1.4.0" - Proxy 配置重构
    ///
    /// 规则：config.version < target_version 时执行
    fn target_version(&self) -> &str;

    /// 执行迁移
    ///
    /// 返回：迁移结果（成功/失败、记录数等）
    async fn execute(&self) -> Result<MigrationResult>;

    /// 回滚迁移（可选实现）
    ///
    /// 默认实现：不支持回滚
    async fn rollback(&self) -> Result<()> {
        Err(anyhow::anyhow!("迁移 {} 不支持回滚", self.id()))
    }
}

/// 迁移结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    /// 迁移 ID
    pub migration_id: String,
    /// 是否成功
    pub success: bool,
    /// 结果消息
    pub message: String,
    /// 迁移的记录数
    pub records_migrated: usize,
    /// 执行时间（秒）
    pub duration_secs: f64,
}

/// 版本比较辅助函数
pub fn compare_versions(v1: &str, v2: &str) -> Ordering {
    use semver::Version;

    let version1 = Version::parse(v1).ok();
    let version2 = Version::parse(v2).ok();

    match (version1, version2) {
        (Some(ver1), Some(ver2)) => ver1.cmp(&ver2),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => v1.cmp(v2), // 字符串比较
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert_eq!(compare_versions("1.3.8", "1.3.9"), Ordering::Less);
        assert_eq!(compare_versions("1.3.9", "1.3.9"), Ordering::Equal);
        assert_eq!(compare_versions("1.4.0", "1.3.9"), Ordering::Greater);
        assert_eq!(compare_versions("2.0.0", "1.9.9"), Ordering::Greater);
    }
}
