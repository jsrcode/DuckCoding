// Migrations - 所有迁移实现
//
// 每个迁移定义目标版本号，按版本号顺序执行

mod profile;
mod proxy_config;
mod session_config;
mod sqlite_to_json;

pub use profile::ProfileMigration;
pub use proxy_config::ProxyConfigMigration;
pub use session_config::SessionConfigMigration;
pub use sqlite_to_json::SqliteToJsonMigration;
