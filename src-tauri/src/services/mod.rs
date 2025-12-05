// 服务层模块
//
// 重组后的目录结构：
// - config: 配置管理（待拆分优化）
// - tool: 工具安装、版本检查、下载
// - proxy: 代理配置和透明代理
// - update: 应用自身更新
// - session: 会话管理（透明代理请求追踪）
// - migration_manager: 统一迁移管理（新）

pub mod config;
pub mod config_watcher;
pub mod migration;
pub mod migration_manager;
pub mod profile_store;
pub mod proxy;
pub mod session;
pub mod tool;
pub mod update;

// 重新导出服务
pub use config::*;
pub use config_watcher::*;
pub use migration::*;
pub use migration_manager::{create_migration_manager, MigrationManager};
pub use profile_store::*;
pub use proxy::*;
// session 模块：明确导出避免 db 名称冲突
pub use session::{manager::SESSION_MANAGER, models::*};
// tool 模块：导出主要服务类和子模块
pub use tool::{
    db::ToolInstanceDB, downloader, downloader::FileDownloader, installer,
    installer::InstallerService, registry::ToolRegistry, version, version::VersionService,
};
pub use update::*;
