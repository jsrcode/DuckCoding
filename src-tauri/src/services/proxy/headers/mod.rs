// Headers 处理器模块 - 为不同工具提供独立的 headers 处理逻辑

use anyhow::Result;
use async_trait::async_trait;
use hyper::HeaderMap as HyperHeaderMap;
use reqwest::header::HeaderMap as ReqwestHeaderMap;

mod claude_processor;
mod codex_processor;
mod gemini_processor;

pub use claude_processor::ClaudeHeadersProcessor;
pub use codex_processor::CodexHeadersProcessor;
pub use gemini_processor::GeminiHeadersProcessor;

/// Headers 处理器 trait
///
/// 为不同的 AI 编程工具提供独立的请求/响应 headers 处理逻辑。
/// 每个工具可能有不同的认证方式、必需的 headers、或特殊的协议要求。
#[async_trait]
pub trait HeadersProcessor: Send + Sync {
    /// 返回工具标识符
    fn tool_id(&self) -> &str;

    /// 处理请求 headers（转发到上游前调用）
    ///
    /// # 参数
    /// - `headers`: 可修改的请求 headers
    /// - `body`: 请求体字节数组（用于签名或内容协商）
    /// - `target_api_key`: 目标 API 的真实密钥
    ///
    /// # 返回
    /// - `Ok(())`: 处理成功
    /// - `Err`: 处理失败（会中断请求）
    async fn process_request(
        &self,
        headers: &mut ReqwestHeaderMap,
        body: &[u8],
        target_api_key: &str,
    ) -> Result<()>;

    /// 处理响应 headers（返回给客户端前调用，可选）
    ///
    /// # 参数
    /// - `headers`: 可修改的响应 headers
    /// - `body`: 响应体字节数组（如果已读取）
    ///
    /// # 默认实现
    /// 默认不处理响应 headers，直接透传
    async fn process_response(
        &self,
        _headers: &mut HyperHeaderMap,
        _body: Option<&[u8]>,
    ) -> Result<()> {
        Ok(())
    }

    /// 是否需要读取响应体
    ///
    /// 如果返回 `true`，代理服务会先读取完整响应体，
    /// 然后调用 `process_response`。
    ///
    /// # 注意
    /// 启用此选项会增加内存使用和延迟，仅在必要时启用。
    fn should_process_response(&self) -> bool {
        false
    }
}

/// 创建 Headers 处理器工厂函数
///
/// # 参数
/// - `tool_id`: 工具标识符 ("claude-code", "codex", "gemini-cli")
///
/// # 返回
/// - 对应工具的 HeadersProcessor 实例
///
/// # Panics
/// 当 `tool_id` 不被支持时会 panic
pub fn create_headers_processor(tool_id: &str) -> Box<dyn HeadersProcessor> {
    match tool_id {
        "claude-code" => Box::new(ClaudeHeadersProcessor),
        "codex" => Box::new(CodexHeadersProcessor),
        "gemini-cli" => Box::new(GeminiHeadersProcessor),
        _ => panic!("Unsupported tool: {}", tool_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_headers_processor() {
        let claude = create_headers_processor("claude-code");
        assert_eq!(claude.tool_id(), "claude-code");

        let codex = create_headers_processor("codex");
        assert_eq!(codex.tool_id(), "codex");

        let gemini = create_headers_processor("gemini-cli");
        assert_eq!(gemini.tool_id(), "gemini-cli");
    }

    #[test]
    #[should_panic(expected = "Unsupported tool")]
    fn test_create_invalid_processor() {
        create_headers_processor("invalid-tool");
    }

    #[tokio::test]
    async fn test_claude_processor_basic() {
        let processor = ClaudeHeadersProcessor;
        let mut headers = ReqwestHeaderMap::new();

        processor
            .process_request(&mut headers, b"", "test-api-key")
            .await
            .unwrap();

        let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
        assert_eq!(auth_header, "Bearer test-api-key");
    }
}
