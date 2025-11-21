// Claude Code Headers 处理器

use super::HeadersProcessor;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::header::HeaderMap as ReqwestHeaderMap;

/// Claude Code 专用 Headers 处理器
///
/// 处理 Anthropic Claude API 的认证和 headers 要求：
/// - 使用 Bearer Token 认证
/// - Authorization header 格式：`Bearer sk-ant-xxx`
pub struct ClaudeHeadersProcessor;

#[async_trait]
impl HeadersProcessor for ClaudeHeadersProcessor {
    fn tool_id(&self) -> &str {
        "claude-code"
    }

    async fn process_request(
        &self,
        headers: &mut ReqwestHeaderMap,
        _body: &[u8],
        target_api_key: &str,
    ) -> Result<()> {
        // 移除客户端提供的认证 headers
        headers.remove("authorization");
        headers.remove("x-api-key");

        // 插入真实的 API Key
        headers.insert(
            "authorization",
            format!("Bearer {}", target_api_key)
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid authorization header: {}", e))?,
        );

        Ok(())
    }

    // Claude Code 不需要特殊的响应处理
    // 使用默认实现即可
}
