// Tool Detector Trait - 工具检测器接口
//
// 定义统一的工具检测、安装、配置管理接口
// 每个工具实现此 trait 以提供工具特定的逻辑

use crate::data::DataManager;
use crate::models::InstallMethod;
use crate::utils::CommandExecutor;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;

/// 工具检测器 Trait
///
/// 每个 AI 开发工具（Claude Code、CodeX、Gemini CLI）都实现此接口
/// 提供检测、安装、配置管理的统一抽象
#[async_trait]
pub trait ToolDetector: Send + Sync {
    // ==================== 基础信息 ====================

    /// 工具唯一标识（如 "claude-code"）
    fn tool_id(&self) -> &str;

    /// 工具显示名称（如 "Claude Code"）
    fn tool_name(&self) -> &str;

    /// 配置目录路径（如 ~/.claude）
    fn config_dir(&self) -> PathBuf;

    /// 配置文件名（如 "settings.json"）
    fn config_file(&self) -> &str;

    /// npm 包名（如 "@anthropic-ai/claude-code"）
    fn npm_package(&self) -> &str;

    /// 版本检查命令（如 "claude --version"）
    fn check_command(&self) -> &str;

    /// 版本检查是否使用代理
    /// - Claude Code: false（代理下会出现 URL 协议错误）
    /// - CodeX/Gemini CLI: true
    fn use_proxy_for_version_check(&self) -> bool;

    // ==================== 检测逻辑 ====================

    /// 检测工具是否已安装
    ///
    /// 默认实现：执行 check_command 并判断是否成功
    async fn is_installed(&self, executor: &CommandExecutor) -> bool {
        let cmd = self.check_command().split_whitespace().next().unwrap_or("");
        if cmd.is_empty() {
            return false;
        }
        executor.command_exists_async(cmd).await
    }

    /// 获取已安装版本
    ///
    /// 默认实现：执行 check_command 并提取版本号
    async fn get_version(&self, executor: &CommandExecutor) -> Option<String> {
        let result = if self.use_proxy_for_version_check() {
            executor.execute_async(self.check_command()).await
        } else {
            self.execute_without_proxy(executor, self.check_command())
                .await
        };

        if result.success {
            self.extract_version_default(&result.stdout)
        } else {
            None
        }
    }

    /// 获取安装路径（如 /usr/local/bin/claude）
    ///
    /// 默认实现：使用 which/where 命令
    async fn get_install_path(&self, executor: &CommandExecutor) -> Option<String> {
        let cmd_name = self.check_command().split_whitespace().next()?;

        #[cfg(target_os = "windows")]
        let which_cmd = format!("where {}", cmd_name);
        #[cfg(not(target_os = "windows"))]
        let which_cmd = format!("which {}", cmd_name);

        let result = executor.execute_async(&which_cmd).await;
        if result.success {
            let path = result.stdout.lines().next()?.trim();
            if !path.is_empty() {
                return Some(path.to_string());
            }
        }
        None
    }

    /// 检测工具的安装方法（npm、Homebrew、官方脚本）
    ///
    /// 需要每个工具自己实现，因为检测逻辑不同
    async fn detect_install_method(&self, executor: &CommandExecutor) -> Option<InstallMethod>;

    // ==================== 安装逻辑 ====================

    /// 安装工具
    ///
    /// 参数：
    /// - executor: 命令执行器
    /// - method: 安装方法（npm/brew/official）
    /// - force: 是否强制重新安装
    async fn install(
        &self,
        executor: &CommandExecutor,
        method: &InstallMethod,
        force: bool,
    ) -> Result<()>;

    /// 更新工具
    ///
    /// 参数：
    /// - executor: 命令执行器
    /// - force: 是否强制更新
    async fn update(&self, executor: &CommandExecutor, force: bool) -> Result<()>;

    // ==================== 配置管理 ====================

    /// 读取工具配置
    ///
    /// 参数：
    /// - manager: 数据管理器（支持 JSON/TOML/ENV）
    async fn read_config(&self, manager: &DataManager) -> Result<Value>;

    /// 保存工具配置
    ///
    /// 参数：
    /// - manager: 数据管理器
    /// - config: 配置内容
    async fn save_config(&self, manager: &DataManager, config: Value) -> Result<()>;

    // ==================== 辅助方法 ====================

    /// 执行命令但不使用代理（用于版本检查）
    ///
    /// 默认实现：移除所有代理环境变量
    async fn execute_without_proxy(
        &self,
        _executor: &CommandExecutor,
        command: &str,
    ) -> crate::utils::CommandResult {
        use crate::utils::platform::PlatformInfo;
        use std::process::Command;

        #[cfg(target_os = "windows")]
        use std::os::windows::process::CommandExt;

        let command_str = command.to_string();
        let platform = PlatformInfo::current();

        tokio::task::spawn_blocking(move || {
            let enhanced_path = platform.build_enhanced_path();

            let output = if platform.is_windows {
                #[cfg(target_os = "windows")]
                {
                    Command::new("cmd")
                        .args(["/C", &command_str])
                        .creation_flags(0x08000000) // CREATE_NO_WINDOW
                        .env("PATH", &enhanced_path)
                        .env_remove("HTTP_PROXY")
                        .env_remove("HTTPS_PROXY")
                        .env_remove("ALL_PROXY")
                        .env_remove("http_proxy")
                        .env_remove("https_proxy")
                        .env_remove("all_proxy")
                        .output()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    Command::new("cmd")
                        .args(["/C", &command_str])
                        .env("PATH", &enhanced_path)
                        .env_remove("HTTP_PROXY")
                        .env_remove("HTTPS_PROXY")
                        .env_remove("ALL_PROXY")
                        .env_remove("http_proxy")
                        .env_remove("https_proxy")
                        .env_remove("all_proxy")
                        .output()
                }
            } else {
                Command::new("sh")
                    .args(["-c", &command_str])
                    .env("PATH", &enhanced_path)
                    .env_remove("HTTP_PROXY")
                    .env_remove("HTTPS_PROXY")
                    .env_remove("ALL_PROXY")
                    .env_remove("http_proxy")
                    .env_remove("https_proxy")
                    .env_remove("all_proxy")
                    .output()
            };

            match output {
                Ok(output) => crate::utils::CommandResult {
                    success: output.status.success(),
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    exit_code: output.status.code(),
                },
                Err(e) => crate::utils::CommandResult {
                    success: false,
                    stdout: String::new(),
                    stderr: e.to_string(),
                    exit_code: None,
                },
            }
        })
        .await
        .unwrap_or_else(|_| crate::utils::CommandResult {
            success: false,
            stdout: String::new(),
            stderr: "执行失败".to_string(),
            exit_code: None,
        })
    }

    /// 默认版本号提取逻辑（正则匹配）
    ///
    /// 匹配格式：v1.2.3 或 1.2.3-beta.1
    fn extract_version_default(&self, output: &str) -> Option<String> {
        let re = regex::Regex::new(r"v?(\d+\.\d+\.\d+(?:-[\w.]+)?)").ok()?;
        re.captures(output)?.get(1).map(|m| m.as_str().to_string())
    }
}
