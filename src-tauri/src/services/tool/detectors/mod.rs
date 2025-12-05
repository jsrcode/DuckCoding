// Tool Detectors Module
//
// 包含所有工具的 Detector 实现和注册表

mod claude_code;
mod codex;
mod gemini_cli;

pub use claude_code::ClaudeCodeDetector;
pub use codex::CodeXDetector;
pub use gemini_cli::GeminiCLIDetector;

use super::detector_trait::ToolDetector;
use std::collections::HashMap;
use std::sync::Arc;

/// Detector 注册表
///
/// 管理所有工具的 Detector 实例，提供统一访问接口
pub struct DetectorRegistry {
    detectors: HashMap<String, Arc<dyn ToolDetector>>,
}

impl DetectorRegistry {
    /// 创建新的注册表并注册所有内置工具
    pub fn new() -> Self {
        let mut registry = Self {
            detectors: HashMap::new(),
        };

        // 注册所有内置工具
        registry.register(Arc::new(ClaudeCodeDetector::new()));
        registry.register(Arc::new(CodeXDetector::new()));
        registry.register(Arc::new(GeminiCLIDetector::new()));

        tracing::debug!(
            "Detector 注册表初始化完成，已注册 {} 个工具",
            registry.detectors.len()
        );

        registry
    }

    /// 注册一个 Detector
    pub fn register(&mut self, detector: Arc<dyn ToolDetector>) {
        let tool_id = detector.tool_id().to_string();
        tracing::trace!("注册工具 Detector: {}", tool_id);
        self.detectors.insert(tool_id, detector);
    }

    /// 根据工具 ID 获取 Detector
    pub fn get(&self, tool_id: &str) -> Option<Arc<dyn ToolDetector>> {
        self.detectors.get(tool_id).cloned()
    }

    /// 获取所有已注册的工具 ID
    pub fn all_tool_ids(&self) -> Vec<String> {
        self.detectors.keys().cloned().collect()
    }

    /// 获取所有 Detector（用于批量操作）
    pub fn all_detectors(&self) -> Vec<Arc<dyn ToolDetector>> {
        self.detectors.values().cloned().collect()
    }

    /// 检查工具是否已注册
    pub fn contains(&self, tool_id: &str) -> bool {
        self.detectors.contains_key(tool_id)
    }
}

impl Default for DetectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = DetectorRegistry::new();
        assert_eq!(registry.detectors.len(), 3);
        assert!(registry.contains("claude-code"));
        assert!(registry.contains("codex"));
        assert!(registry.contains("gemini-cli"));
    }

    #[test]
    fn test_get_detector() {
        let registry = DetectorRegistry::new();

        let claude_detector = registry.get("claude-code");
        assert!(claude_detector.is_some());
        assert_eq!(claude_detector.unwrap().tool_name(), "Claude Code");

        let codex_detector = registry.get("codex");
        assert!(codex_detector.is_some());
        assert_eq!(codex_detector.unwrap().tool_name(), "CodeX");

        let gemini_detector = registry.get("gemini-cli");
        assert!(gemini_detector.is_some());
        assert_eq!(gemini_detector.unwrap().tool_name(), "Gemini CLI");
    }

    #[test]
    fn test_all_tool_ids() {
        let registry = DetectorRegistry::new();
        let ids = registry.all_tool_ids();

        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"claude-code".to_string()));
        assert!(ids.contains(&"codex".to_string()));
        assert!(ids.contains(&"gemini-cli".to_string()));
    }
}
