use crate::models::{InstallMethod, Tool};
use crate::services::tool::DetectorRegistry;
use anyhow::Result;

/// 安装服务（新架构：委托给 Detector）
pub struct InstallerService {
    detector_registry: DetectorRegistry,
    command_executor: crate::utils::CommandExecutor,
}

impl InstallerService {
    pub fn new() -> Self {
        InstallerService {
            detector_registry: DetectorRegistry::new(),
            command_executor: crate::utils::CommandExecutor::new(),
        }
    }

    /// 安装工具（委托给 Detector）
    pub async fn install(&self, tool: &Tool, method: &InstallMethod, force: bool) -> Result<()> {
        let detector = self
            .detector_registry
            .get(&tool.id)
            .ok_or_else(|| anyhow::anyhow!("未知的工具 ID: {}", tool.id))?;

        tracing::info!("使用 Detector 安装工具: {}", tool.name);
        detector
            .install(&self.command_executor, method, force)
            .await
    }

    /// 更新工具（委托给 Detector）
    pub async fn update(&self, tool: &Tool, force: bool) -> Result<()> {
        let detector = self
            .detector_registry
            .get(&tool.id)
            .ok_or_else(|| anyhow::anyhow!("未知的工具 ID: {}", tool.id))?;

        tracing::info!("使用 Detector 更新工具: {}", tool.name);
        detector.update(&self.command_executor, force).await
    }

    /// 检查工具是否已安装（委托给 Detector）
    pub async fn is_installed(&self, tool: &Tool) -> bool {
        if let Some(detector) = self.detector_registry.get(&tool.id) {
            detector.is_installed(&self.command_executor).await
        } else {
            false
        }
    }

    /// 获取已安装版本（委托给 Detector）
    pub async fn get_installed_version(&self, tool: &Tool) -> Option<String> {
        if let Some(detector) = self.detector_registry.get(&tool.id) {
            detector.get_version(&self.command_executor).await
        } else {
            None
        }
    }
}

impl Default for InstallerService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_creation() {
        let service = InstallerService::new();
        // 验证 detector_registry 已初始化
        assert!(service.detector_registry.contains("claude-code"));
        assert!(service.detector_registry.contains("codex"));
        assert!(service.detector_registry.contains("gemini-cli"));
    }
}
