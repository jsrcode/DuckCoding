// Codex Headers 处理器

use super::HeadersProcessor;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::header::HeaderMap as ReqwestHeaderMap;

/// Codex 专用 Headers 处理器
///
/// 处理 OpenAI API 的认证和 headers 要求：
/// - 使用 Bearer Token 认证
/// - Authorization header 格式：`Bearer sk-xxx`
/// - 可能需要额外的 OpenAI-Organization 或 OpenAI-Project headers
///
/// # TODO
/// 根据实际需求添加：
/// - OpenAI-Organization header 处理
/// - OpenAI-Project header 处理
/// - 特殊的内容类型处理
pub struct CodexHeadersProcessor;

#[async_trait]
impl HeadersProcessor for CodexHeadersProcessor {
    fn tool_id(&self) -> &str {
        "codex"
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

        // 插入真实的 OpenAI API Key（Bearer Token 格式）
        headers.insert(
            "authorization",
            format!("Bearer {}", target_api_key)
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid authorization header: {}", e))?,
        );

        // TODO: 根据需要添加其他 OpenAI 特定的 headers
        // 例如：
        // if let Some(org_id) = get_organization_id() {
        //     headers.insert("OpenAI-Organization", org_id.parse()?);
        // }

        Ok(())
    }

    // Codex 当前不需要特殊的响应处理
    // 如果未来需要（例如处理速率限制信息），可以在此实现
}
