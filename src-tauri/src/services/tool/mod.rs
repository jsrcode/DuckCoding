// 工具服务模块
//
// 包含工具的安装、版本检查、下载等功能

pub mod db;
pub mod detector_trait;
pub mod detectors;
pub mod downloader;
pub mod installer;
pub mod registry;
pub mod tools_config;
pub mod version;

pub use db::ToolInstanceDB;
pub use detector_trait::ToolDetector;
pub use detectors::{ClaudeCodeDetector, CodeXDetector, DetectorRegistry, GeminiCLIDetector};
pub use downloader::FileDownloader;
pub use installer::InstallerService;
pub use registry::ToolRegistry;
pub use tools_config::{LocalToolInstance, SSHToolInstance, ToolGroup, ToolsConfig, WSLToolInstance};
pub use version::VersionService;
