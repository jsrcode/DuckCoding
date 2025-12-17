// 托盘菜单和窗口管理
pub mod tray;

// 启动初始化逻辑
pub mod initialization;

// 重新导出常用函数供 main.rs 使用
pub use initialization::initialize_app;
pub use tray::focus_main_window;
