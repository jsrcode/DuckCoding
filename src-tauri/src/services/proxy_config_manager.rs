//! 透明代理配置管理器

use crate::data::DataManager;
use crate::models::proxy_config::ProxyStore;
use crate::models::proxy_config::ToolProxyConfig;
use anyhow::{Context, Result};
use std::path::PathBuf;

pub struct ProxyConfigManager {
    data_manager: DataManager,
    proxy_path: PathBuf,
}

impl ProxyConfigManager {
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("无法获取用户主目录"))?;
        let duckcoding_dir = home_dir.join(".duckcoding");
        std::fs::create_dir_all(&duckcoding_dir)?;

        Ok(Self {
            data_manager: DataManager::new(),
            proxy_path: duckcoding_dir.join("proxy.json"),
        })
    }

    /// 加载 proxy.json
    pub fn load_proxy_store(&self) -> Result<ProxyStore> {
        if !self.proxy_path.exists() {
            return Ok(ProxyStore::new());
        }

        let value = self
            .data_manager
            .json()
            .read(&self.proxy_path)
            .context("读取 proxy.json 失败")?;

        serde_json::from_value(value).context("反序列化 ProxyStore 失败")
    }

    /// 保存 proxy.json
    pub fn save_proxy_store(&self, store: &ProxyStore) -> Result<()> {
        let value = serde_json::to_value(store)?;
        self.data_manager
            .json()
            .write(&self.proxy_path, &value)
            .map_err(Into::into)
    }

    /// 获取指定工具的代理配置
    pub fn get_config(&self, tool_id: &str) -> Result<Option<ToolProxyConfig>> {
        let store = self.load_proxy_store()?;
        Ok(store.get_config(tool_id).cloned())
    }

    /// 更新指定工具的代理配置
    pub fn update_config(&self, tool_id: &str, config: ToolProxyConfig) -> Result<()> {
        let mut store = self.load_proxy_store()?;
        store.update_config(tool_id, config);
        self.save_proxy_store(&store)
    }

    /// 删除指定工具的代理配置（重置为默认）
    pub fn reset_config(&self, tool_id: &str) -> Result<()> {
        let mut store = self.load_proxy_store()?;
        let default_port = ToolProxyConfig::default_port(tool_id);
        store.update_config(tool_id, ToolProxyConfig::new(default_port));
        self.save_proxy_store(&store)
    }

    /// 获取所有工具的配置
    pub fn get_all_configs(&self) -> Result<ProxyStore> {
        self.load_proxy_store()
    }
}

impl Default for ProxyConfigManager {
    fn default() -> Self {
        Self::new().expect("创建 ProxyConfigManager 失败")
    }
}
