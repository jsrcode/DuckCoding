//! 代理配置辅助模块
//!
//! 提供代理配置的高级操作，避免 utils 层依赖 services 层

use crate::services::proxy::ProxyService;
use crate::utils::config::read_global_config;
use anyhow::Result;

/// 应用全局配置中的代理设置到环境变量
///
/// 读取 GlobalConfig，如果配置了 HTTP/HTTPS/SOCKS5 代理，
/// 则应用到当前进程的环境变量中
///
/// # 使用场景
/// - 应用启动时
/// - 保存代理配置后
/// - 工具安装/更新时需要代理
pub fn apply_global_proxy() -> Result<()> {
    if let Ok(Some(config)) = read_global_config() {
        ProxyService::apply_proxy_from_config(&config);
        tracing::debug!("已应用全局代理配置到环境变量");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_global_proxy() {
        // 测试无配置文件时不会 panic
        let result = apply_global_proxy();
        assert!(result.is_ok());
    }
}
