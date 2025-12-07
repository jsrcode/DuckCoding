//! Profile 管理模块（v2.1 - 简化版）
//!
//! 设计原则：工具分组即类型
//! - profiles.json: 使用具体类型（ClaudeProfile/CodexProfile/GeminiProfile）
//! - active.json: 激活状态管理

mod manager;
mod native_config;
mod types;

pub use manager::ProfileManager;
pub use types::{
    ActiveMetadata, ActiveProfile, ActiveStore, ClaudeProfile, CodexProfile, GeminiProfile,
    ProfileDescriptor, ProfilesMetadata, ProfilesStore,
};
