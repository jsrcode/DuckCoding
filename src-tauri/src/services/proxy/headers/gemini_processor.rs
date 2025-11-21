// Gemini CLI Headers 处理器

use super::HeadersProcessor;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::header::HeaderMap as ReqwestHeaderMap;

/// Gemini CLI 专用 Headers 处理器
///
/// 处理 Google Gemini API 的认证和 headers 要求：
/// - 使用 x-goog-api-key header 认证
/// - API Key 格式：直接的 key 字符串（不需要 Bearer 前缀）
/// - 可能需要额外的 Google Cloud 相关 headers
///
/// # TODO
/// 根据实际需求添加：
/// - x-goog-user-project header 处理（计费项目）
/// - 特殊的 Content-Type 或 Accept headers
/// - OAuth 2.0 令牌支持（如果 Gemini CLI 使用 OAuth）
pub struct GeminiHeadersProcessor;

#[async_trait]
impl HeadersProcessor for GeminiHeadersProcessor {
    fn tool_id(&self) -> &str {
        "gemini-cli"
    }

    async fn process_request(
        &self,
        headers: &mut ReqwestHeaderMap,
        _body: &[u8],
        target_api_key: &str,
    ) -> Result<()> {
        // 移除客户端提供的认证 headers
        headers.remove("x-goog-api-key");
        headers.remove("authorization");
        headers.remove("x-api-key");

        // 插入真实的 Google API Key
        // Google APIs 通常使用 x-goog-api-key header
        headers.insert(
            "x-goog-api-key",
            target_api_key
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid x-goog-api-key header: {}", e))?,
        );

        // TODO: 根据需要添加其他 Google 特定的 headers
        // 例如：
        // if let Some(project_id) = get_project_id() {
        //     headers.insert("x-goog-user-project", project_id.parse()?);
        // }

        Ok(())
    }

    // Gemini CLI 当前不需要特殊的响应处理
    // 如果未来需要（例如处理配额信息），可以在此实现
}
