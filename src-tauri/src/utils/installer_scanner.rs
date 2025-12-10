// 安装器路径扫描工具
//
// 从工具路径智能扫描安装器路径（npm、brew 等）

use crate::models::InstallMethod;
use crate::utils::PlatformInfo;
use std::path::PathBuf;

/// 工具候选结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolCandidate {
    /// 工具可执行文件路径
    pub tool_path: String,
    /// 安装器路径
    pub installer_path: Option<String>,
    /// 安装方法
    pub install_method: InstallMethod,
    /// 版本号
    pub version: String,
}

/// 安装器候选结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct InstallerCandidate {
    /// 安装器路径
    pub path: String,
    /// 安装器类型
    pub installer_type: InstallMethod,
    /// 扫描级别（1=同级目录, 2=上级目录）
    pub level: u8,
}

/// 从工具路径扫描安装器
///
/// 策略：
/// 1. 第一级：工具所在目录
/// 2. 第二级：上级目录
///
/// 示例：
/// - 工具：~/.nvm/versions/node/v18.16.0/bin/gemini
/// - 第一级扫描：~/.nvm/versions/node/v18.16.0/bin/
/// - 第二级扫描：~/.nvm/versions/node/v18.16.0/
pub fn scan_installer_paths(tool_path: &str) -> Vec<InstallerCandidate> {
    let mut candidates = Vec::new();
    let tool_path_buf = PathBuf::from(tool_path);

    // 1. 获取工具所在目录
    let tool_dir = match tool_path_buf.parent() {
        Some(dir) => dir,
        None => return candidates,
    };

    // 2. 定义安装器名称（包含所有可能的扩展名）
    let installer_configs = [
        ("npm", InstallMethod::Npm),
        ("npm.cmd", InstallMethod::Npm),
        ("npm.exe", InstallMethod::Npm),
        ("pnpm", InstallMethod::Npm),
        ("pnpm.cmd", InstallMethod::Npm),
        ("pnpm.exe", InstallMethod::Npm),
        ("yarn", InstallMethod::Npm),
        ("yarn.cmd", InstallMethod::Npm),
        ("yarn.exe", InstallMethod::Npm),
        ("brew", InstallMethod::Brew),
    ];

    // 3. 第一级扫描：工具所在目录
    for (name, installer_type) in &installer_configs {
        let installer_path = tool_dir.join(name);
        if installer_path.is_file() {
            candidates.push(InstallerCandidate {
                path: installer_path.to_string_lossy().to_string(),
                installer_type: installer_type.clone(),
                level: 1,
            });
        }
    }

    // 4. 第二级扫描：上级目录
    if let Some(parent_dir) = tool_dir.parent() {
        for (name, installer_type) in &installer_configs {
            let installer_path = parent_dir.join(name);
            if installer_path.is_file() {
                // 避免重复：检查路径是否已在候选列表中
                let path_str = installer_path.to_string_lossy().to_string();
                if !candidates.iter().any(|c| c.path == path_str) {
                    candidates.push(InstallerCandidate {
                        path: path_str,
                        installer_type: installer_type.clone(),
                        level: 2,
                    });
                }
            }
        }
    }

    // 5. 排序：同级 npm > 上级 npm > 同级 brew > 上级 brew
    candidates.sort_by_key(|c| {
        let type_priority = match c.installer_type {
            InstallMethod::Npm => 1,
            InstallMethod::Brew => 2,
            _ => 3,
        };
        (c.level, type_priority)
    });

    candidates
}

/// 扫描所有可能的工具实例（用于自动扫描）
///
/// 工作流程：
/// 1. 获取硬编码路径列表
/// 2. 在每个路径中查找工具可执行文件
/// 3. 返回所有找到的工具路径
///
/// 注意：版本检测和安装器扫描在命令层完成
pub fn scan_tool_executables(tool_id: &str) -> Vec<String> {
    let platform = PlatformInfo::current();
    let search_paths_str = platform.build_enhanced_path();

    // 解析 PATH 环境变量
    let separator = platform.path_separator();
    let search_paths: Vec<&str> = search_paths_str.split(separator).collect();

    // 工具ID到可执行文件名的映射
    let executable_name = match tool_id {
        "claude-code" => "claude",
        "gemini-cli" => "gemini",
        "codex" => "codex",
        _ => tool_id, // 默认使用 tool_id
    };

    // 工具可执行文件名（包含扩展名）
    let tool_names = if cfg!(target_os = "windows") {
        vec![
            format!("{}.cmd", executable_name),
            format!("{}.exe", executable_name),
            format!("{}.bat", executable_name),
            executable_name.to_string(),
        ]
    } else {
        vec![executable_name.to_string()]
    };

    let mut found_paths = Vec::new();

    // 在所有路径中查找工具
    for search_path in search_paths {
        let search_dir = PathBuf::from(search_path);
        if !search_dir.is_dir() {
            continue;
        }

        for tool_name in &tool_names {
            let tool_path = search_dir.join(tool_name);
            if tool_path.is_file() {
                let path_str = tool_path.to_string_lossy().to_string();
                // 避免重复
                if !found_paths.contains(&path_str) {
                    found_paths.push(path_str);
                }
            }
        }
    }

    found_paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_installer_paths() {
        // 测试路径扫描逻辑
        // 注意：实际测试需要在真实文件系统中进行
        let tool_path = "/usr/local/bin/claude";
        let candidates = scan_installer_paths(tool_path);
        // 应该在 /usr/local/bin/ 和 /usr/local/ 中查找
        println!("Found {} candidates", candidates.len());
    }
}
