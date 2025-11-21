// filepath: e:\DuckCoding\src-tauri\src\models\config.rs

// 全局配置结构，移动到 models 以便在库和二进制之间共享
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 单个工具的透明代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProxyConfig {
    pub enabled: bool,
    pub port: u16,
    pub local_api_key: Option<String>,    // 保护密钥
    pub real_api_key: Option<String>,     // 备份的真实 API Key
    pub real_base_url: Option<String>,    // 备份的真实 Base URL
    pub real_model_provider: Option<String>, // 备份的 model_provider (Codex 专用)
    pub allow_public: bool,
}

impl Default for ToolProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 8787,
            local_api_key: None,
            real_api_key: None,
            real_base_url: None,
            real_model_provider: None,
            allow_public: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GlobalConfig {
    pub user_id: String,
    pub system_token: String,
    #[serde(default)]
    pub proxy_enabled: bool,
    #[serde(default)]
    pub proxy_type: Option<String>, // "http", "https", "socks5"
    #[serde(default)]
    pub proxy_host: Option<String>,
    #[serde(default)]
    pub proxy_port: Option<String>,
    #[serde(default)]
    pub proxy_username: Option<String>,
    #[serde(default)]
    pub proxy_password: Option<String>,
    #[serde(default)]
    pub proxy_bypass_urls: Vec<String>, // 代理过滤URL列表
    // 透明代理功能 (实验性)
    #[serde(default)]
    pub transparent_proxy_enabled: bool,
    #[serde(default = "default_transparent_proxy_port")]
    pub transparent_proxy_port: u16,
    #[serde(default)]
    pub transparent_proxy_api_key: Option<String>, // 用于保护本地代理的 API Key
    // 保存真实的 ClaudeCode API 配置（透明代理启用时使用）
    #[serde(default)]
    pub transparent_proxy_real_api_key: Option<String>,
    #[serde(default)]
    pub transparent_proxy_real_base_url: Option<String>,
    // 允许局域网访问透明代理（默认仅本地访问）
    #[serde(default)]
    pub transparent_proxy_allow_public: bool,
    // 多工具透明代理配置（新架构）
    #[serde(default = "default_proxy_configs")]
    pub proxy_configs: HashMap<String, ToolProxyConfig>,
}

fn default_transparent_proxy_port() -> u16 {
    8787
}

fn default_proxy_configs() -> HashMap<String, ToolProxyConfig> {
    let mut configs = HashMap::new();

    configs.insert(
        "claude-code".to_string(),
        ToolProxyConfig {
            enabled: false,
            port: 8787,
            local_api_key: None,
            real_api_key: None,
            real_base_url: None,
            real_model_provider: None,
            allow_public: false,
        },
    );

    configs.insert(
        "codex".to_string(),
        ToolProxyConfig {
            enabled: false,
            port: 8788,
            local_api_key: None,
            real_api_key: None,
            real_base_url: None,
            real_model_provider: None,
            allow_public: false,
        },
    );

    configs.insert(
        "gemini-cli".to_string(),
        ToolProxyConfig {
            enabled: false,
            port: 8789,
            local_api_key: None,
            real_api_key: None,
            real_base_url: None,
            real_model_provider: None,
            allow_public: false,
        },
    );

    configs
}

impl GlobalConfig {
    /// 获取指定工具的代理配置
    pub fn get_proxy_config(&self, tool_id: &str) -> Option<&ToolProxyConfig> {
        self.proxy_configs.get(tool_id)
    }

    /// 获取指定工具的可变代理配置
    pub fn get_proxy_config_mut(&mut self, tool_id: &str) -> Option<&mut ToolProxyConfig> {
        self.proxy_configs.get_mut(tool_id)
    }

    /// 确保工具的代理配置存在（如果不存在则创建默认配置）
    pub fn ensure_proxy_config(&mut self, tool_id: &str, default_port: u16) {
        self.proxy_configs
            .entry(tool_id.to_string())
            .or_insert_with(|| ToolProxyConfig {
                enabled: false,
                port: default_port,
                local_api_key: None,
                real_api_key: None,
                real_base_url: None,
                real_model_provider: None,
                allow_public: false,
            });
    }
}
