# DataManager 统一数据管理系统

> 为 DuckCoding 项目提供统一的数据管理接口，支持 JSON、TOML、ENV、SQLite 四种格式

## 📚 目录

- [快速开始](#快速开始)
- [API 参考](#api-参考)
- [使用场景](#使用场景)
- [最佳实践](#最佳实践)
- [迁移指南](#迁移指南)
- [架构设计](#架构设计)

## 🚀 快速开始

### 基本使用

```rust
use crate::data::DataManager;
use std::path::Path;

// 创建管理器实例
let manager = DataManager::new();

// 读取 JSON 配置（带缓存）
let config = manager.json().read(Path::new("config.json"))?;

// 写入 JSON 配置
manager.json().write(Path::new("config.json"), &config)?;
```

### 四种操作模式

```rust
// 1. 带缓存的 JSON 操作（用于全局配置和 Profile）
let config = manager.json().read(path)?;

// 2. 无缓存的 JSON 操作（用于工具原生配置，需实时更新）
let settings = manager.json_uncached().read(path)?;

// 3. TOML 操作（保留注释和格式）
let doc = manager.toml().read_document(path)?;
manager.toml().write(path, &doc)?;

// 4. ENV 文件操作（自动排序和格式化）
let env_vars = manager.env().read(path)?;
manager.env().write(path, &env_vars)?;

// 5. SQLite 操作（带连接池和查询缓存）
let db = manager.sqlite(Path::new("app.db"))?;
let rows = db.query("SELECT * FROM users WHERE id = ?", &["1"])?;
```

## 📖 API 参考

### DataManager

统一入口，提供各格式管理器的访问。

```rust
impl DataManager {
    /// 创建新的 DataManager 实例（使用默认缓存配置）
    pub fn new() -> Self

    /// 创建带自定义缓存配置的实例
    pub fn with_cache_config(config: CacheConfig) -> Self

    /// 获取带缓存的 JSON 管理器
    pub fn json(&self) -> JsonManager<'_>

    /// 获取无缓存的 JSON 管理器
    pub fn json_uncached(&self) -> JsonManager<'_>

    /// 获取 TOML 管理器
    pub fn toml(&self) -> TomlManager<'_>

    /// 获取 ENV 管理器
    pub fn env(&self) -> EnvManager
}
```

### JsonManager

JSON 格式管理器，支持 `serde_json::Value` 的读写。

```rust
impl JsonManager<'_> {
    /// 读取 JSON 文件
    ///
    /// 返回 `serde_json::Value`
    /// 根据是否启用缓存自动处理缓存逻辑
    pub fn read(&self, path: &Path) -> Result<Value>

    /// 写入 JSON 文件
    ///
    /// - 自动创建父目录
    /// - 自动设置 Unix 权限（0o600）
    /// - 使用原子写入（临时文件 + rename）
    /// - 自动失效缓存
    pub fn write(&self, path: &Path, value: &Value) -> Result<()>
}
```

### TomlManager

TOML 格式管理器，支持保留注释和格式。

```rust
impl TomlManager<'_> {
    /// 读取 TOML 文件为 toml::Value（会丢失注释）
    pub fn read(&self, path: &Path) -> Result<TomlValue>

    /// 读取 TOML 文件为 DocumentMut（保留注释和格式）
    pub fn read_document(&self, path: &Path) -> Result<DocumentMut>

    /// 写入 TOML 文件（保留格式）
    pub fn write(&self, path: &Path, doc: &DocumentMut) -> Result<()>
}
```

### EnvManager

ENV 文件管理器，提供键值对的读写。

```rust
impl EnvManager {
    /// 读取 .env 文件
    ///
    /// 返回 HashMap<String, String>
    /// 自动跳过空行和注释
    pub fn read(&self, path: &Path) -> Result<HashMap<String, String>>

    /// 写入 .env 文件
    ///
    /// - 自动按键名排序
    /// - 格式：KEY=VALUE
    /// - 自动创建父目录和设置权限
    pub fn write(&self, path: &Path, vars: &HashMap<String, String>) -> Result<()>
}
```

### SqliteManager

SQLite 数据库管理器，提供查询缓存和事务支持。

```rust
impl SqliteManager {
    /// 创建带缓存的管理器
    pub fn with_cache(path: &Path, capacity: usize, ttl: Duration) -> Result<Self>

    /// 创建无缓存的管理器
    pub fn without_cache(path: &Path) -> Result<Self>

    /// 执行查询（返回通用 JSON 格式行）
    ///
    /// 自动缓存查询结果，基于 SQL + 参数
    pub fn query(&self, sql: &str, params: &[&str]) -> Result<Vec<QueryRow>>

    /// 执行更新/插入/删除
    ///
    /// 自动失效相关表的缓存
    /// 返回受影响的行数
    pub fn execute(&self, sql: &str, params: &[&str]) -> Result<usize>

    /// 执行批量更新
    pub fn execute_batch(&self, sql: &str, params_list: &[Vec<String>]) -> Result<Vec<usize>>

    /// 执行事务
    ///
    /// 事务提交后自动清空所有缓存
    pub fn transaction<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Transaction) -> Result<T>

    /// 执行原始 SQL（用于 DDL 等操作）
    pub fn execute_raw(&self, sql: &str) -> Result<()>

    /// 检查表是否存在
    pub fn table_exists(&self, table_name: &str) -> Result<bool>

    /// 清空查询缓存
    pub fn clear_cache(&self)

    /// 使指定表的缓存失效
    pub fn invalidate_table(&self, table_name: &str)
}

/// 查询结果行（通用 JSON 格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRow {
    pub columns: Vec<String>,
    pub values: Vec<serde_json::Value>,
}
```

## 🎯 使用场景

### 场景 1：读写全局配置

全局配置不频繁变化，适合使用缓存。

```rust
use crate::data::DataManager;
use crate::utils::config::global_config_path;

pub fn read_global_config() -> Result<Option<GlobalConfig>> {
    let config_path = global_config_path()?;
    if !config_path.exists() {
        return Ok(None);
    }

    let manager = DataManager::new();
    let config_value = manager
        .json()  // 使用带缓存的管理器
        .read(&config_path)?;

    let config: GlobalConfig = serde_json::from_value(config_value)?;
    Ok(Some(config))
}

pub fn write_global_config(config: &GlobalConfig) -> Result<()> {
    let config_path = global_config_path()?;
    let manager = DataManager::new();
    let config_value = serde_json::to_value(config)?;

    manager.json().write(&config_path, &config_value)?;
    Ok(())
}
```

### 场景 2：读写工具原生配置

工具配置可能被外部修改，需要实时读取。

```rust
use crate::data::DataManager;

pub fn read_claude_settings() -> Result<Value> {
    let tool = Tool::claude_code();
    let config_path = tool.config_dir.join(&tool.config_file);

    if !config_path.exists() {
        return Ok(Value::Object(Map::new()));
    }

    let manager = DataManager::new();
    let settings = manager
        .json_uncached()  // 使用无缓存管理器
        .read(&config_path)?;

    Ok(settings)
}

pub fn save_claude_settings(settings: &Value) -> Result<()> {
    let tool = Tool::claude_code();
    let config_path = tool.config_dir.join(&tool.config_file);

    let manager = DataManager::new();
    manager
        .json_uncached()
        .write(&config_path, settings)?;

    Ok(())
}
```

### 场景 3：TOML 配置（保留注释）

Codex 的 config.toml 需要保留用户的注释和格式。

```rust
use crate::data::DataManager;
use toml_edit::DocumentMut;

pub fn update_codex_config(api_key: &str, base_url: &str) -> Result<()> {
    let config_path = tool.config_dir.join("config.toml");
    let manager = DataManager::new();

    // 读取现有配置（保留注释）
    let mut doc = if config_path.exists() {
        manager.toml().read_document(&config_path)?
    } else {
        DocumentMut::new()
    };

    // 更新字段
    doc["model_provider"] = toml_edit::value("duckcoding");

    // 写回（保留注释和格式）
    manager.toml().write(&config_path, &doc)?;
    Ok(())
}
```

### 场景 4：ENV 文件管理

Gemini CLI 使用 .env 文件存储配置。

```rust
use crate::data::DataManager;
use std::collections::HashMap;

pub fn update_gemini_env(api_key: &str, base_url: &str) -> Result<()> {
    let env_path = tool.config_dir.join(".env");
    let manager = DataManager::new();

    // 读取现有环境变量
    let mut env_vars = if env_path.exists() {
        manager.env().read(&env_path)?
    } else {
        HashMap::new()
    };

    // 更新字段
    env_vars.insert("GEMINI_API_KEY".to_string(), api_key.to_string());
    env_vars.insert("GOOGLE_GEMINI_BASE_URL".to_string(), base_url.to_string());

    // 写回（自动排序）
    manager.env().write(&env_path, &env_vars)?;
    Ok(())
}
```

### 场景 5：SQLite 数据库操作

使用 SQLite 存储工具实例、会话记录等结构化数据。

```rust
use crate::data::DataManager;
use std::path::Path;

pub fn manage_tool_instances() -> Result<()> {
    let manager = DataManager::new();
    let db = manager.sqlite(Path::new("~/.duckcoding/tools.db"))?;

    // 创建表（仅首次）
    if !db.table_exists("tool_instances")? {
        db.execute_raw(
            "CREATE TABLE tool_instances (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                type TEXT NOT NULL,
                version TEXT,
                created_at INTEGER
            )"
        )?;
    }

    // 插入数据
    db.execute(
        "INSERT INTO tool_instances (id, name, type, version, created_at) VALUES (?, ?, ?, ?, ?)",
        &["claude-1", "Claude Code", "local", "0.24.0", &chrono::Utc::now().timestamp().to_string()]
    )?;

    // 查询数据（自动缓存）
    let rows = db.query("SELECT * FROM tool_instances WHERE type = ?", &["local"])?;
    for row in rows {
        println!("Found: {:?}", row.values);
    }

    // 使用事务
    db.transaction(|tx| {
        tx.execute("UPDATE tool_instances SET version = ? WHERE id = ?", ["0.25.0", "claude-1"])?;
        tx.execute("INSERT INTO logs (tool_id, message) VALUES (?, ?)", ["claude-1", "Updated version"])?;
        Ok(())
    })?;

    Ok(())
}

// 连接池自动复用
pub fn reuse_connection() -> Result<()> {
    let manager = DataManager::new();

    // 第一次获取连接
    let db1 = manager.sqlite(Path::new("app.db"))?;
    db1.execute("INSERT INTO users (name) VALUES (?)", &["Alice"])?;

    // 第二次获取相同路径的连接（复用）
    let db2 = manager.sqlite(Path::new("app.db"))?;
    let rows = db2.query("SELECT * FROM users", &[])?;

    Ok(())
}
```

## 💡 最佳实践

### 1. 选择合适的缓存策略

```rust
// ✅ 好：全局配置使用缓存
let config = manager.json().read(global_config_path)?;

// ✅ 好：工具配置不使用缓存
let settings = manager.json_uncached().read(tool_settings_path)?;

// ❌ 差：工具配置使用缓存（可能读到过期数据）
let settings = manager.json().read(tool_settings_path)?;
```

### 2. TOML 格式处理

```rust
// ✅ 好：需要保留注释时使用 read_document()
let doc = manager.toml().read_document(path)?;
manager.toml().write(path, &doc)?;

// ⚠️  注意：read() 会丢失注释，仅用于转 JSON
let value = manager.toml().read(path)?;
let json = serde_json::to_value(&value)?;
```

### 3. 错误处理

```rust
// ✅ 好：提供上下文信息
manager
    .json()
    .read(&path)
    .with_context(|| format!("读取配置失败: {path:?}"))?;

// ❌ 差：吞噬错误
manager.json().read(&path).ok();
```

### 4. 路径处理

```rust
// ✅ 好：使用 Path/PathBuf
let path = config_dir.join("settings.json");
manager.json().write(&path, &value)?;

// ❌ 差：使用字符串拼接
let path_str = format!("{}/settings.json", config_dir);
```

### 5. 复用 DataManager 实例

```rust
// ✅ 好：在函数内创建
pub fn process_configs() -> Result<()> {
    let manager = DataManager::new();
    manager.json().read(path1)?;
    manager.json().write(path2, &value)?;
    Ok(())
}

// ⚠️  注意：DataManager 是轻量级的，可以多次创建
// 但在同一函数内建议复用实例
```

### 6. SQLite 使用建议

```rust
// ✅ 好：使用连接池自动复用
let manager = DataManager::new();
let db1 = manager.sqlite(Path::new("app.db"))?;  // 创建连接
let db2 = manager.sqlite(Path::new("app.db"))?;  // 复用连接

// ✅ 好：使用事务确保原子性
db.transaction(|tx| {
    tx.execute("UPDATE users SET balance = balance - 100 WHERE id = ?", ["1"])?;
    tx.execute("UPDATE users SET balance = balance + 100 WHERE id = ?", ["2"])?;
    Ok(())
})?;

// ✅ 好：利用查询缓存
let rows = db.query("SELECT * FROM users", &[])?;  // 缓存查询结果
let rows2 = db.query("SELECT * FROM users", &[])?; // 命中缓存

// ⚠️  注意：写操作后相关表的缓存会自动失效
db.execute("INSERT INTO users (name) VALUES (?)", &["Alice"])?;
// users 表的查询缓存已自动清空

// ❌ 差：忘记使用 table_exists 检查
db.execute_raw("CREATE TABLE users (...)")?;  // 表已存在时会报错

// ✅ 好：先检查表是否存在
if !db.table_exists("users")? {
    db.execute_raw("CREATE TABLE users (...)")?;
}
```

## 🔄 迁移指南

### 从直接文件操作迁移

**迁移前：**

```rust
// 读取 JSON
let content = fs::read_to_string(&path)?;
let config: Config = serde_json::from_str(&content)?;

// 写入 JSON
let json = serde_json::to_string_pretty(&config)?;
fs::write(&path, json)?;
```

**迁移后：**

```rust
let manager = DataManager::new();

// 读取 JSON
let json_value = manager.json().read(&path)?;
let config: Config = serde_json::from_value(json_value)?;

// 写入 JSON
let json_value = serde_json::to_value(&config)?;
manager.json().write(&path, &json_value)?;
```

### ENV 文件处理简化

**迁移前（20+ 行）：**

```rust
fn read_env_pairs(path: &Path) -> Result<HashMap<String, String>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(path)?;
    let mut vars = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = trimmed.split_once('=') {
            vars.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    Ok(vars)
}
```

**迁移后（3 行）：**

```rust
fn read_env_pairs(path: &Path) -> Result<HashMap<String, String>> {
    let manager = DataManager::new();
    manager.env().read(path).map_err(|e| anyhow::anyhow!(e))
}
```

### 常见模式映射表

| 旧代码                                        | 新代码                            | 说明                         |
| --------------------------------------------- | --------------------------------- | ---------------------------- |
| `fs::read_to_string` + `serde_json::from_str` | `manager.json().read()`           | JSON 读取                    |
| `serde_json::to_string_pretty` + `fs::write`  | `manager.json().write()`          | JSON 写入                    |
| `fs::read_to_string` + `toml::from_str`       | `manager.toml().read()`           | TOML 读取（丢失注释）        |
| `toml_edit` 手动解析                          | `manager.toml().read_document()`  | TOML 读取（保留注释）        |
| 手动解析 .env                                 | `manager.env().read()`            | ENV 读取                     |
| 手动拼接 KEY=VALUE                            | `manager.env().write()`           | ENV 写入                     |
| `fs::create_dir_all` + `fs::write`            | `manager.*.write()`               | 目录自动创建                 |
| `rusqlite::Connection::open`                  | `manager.sqlite(path)?`           | SQLite 连接（带连接池）      |
| 手动执行 SQL + 解析结果                       | `db.query(sql, params)?`          | SQLite 查询（带缓存）        |
| 手动事务管理                                  | `db.transaction(\|tx\| { ... })?` | SQLite 事务（自动提交/回滚） |

### SQLite 迁移示例

**迁移前（直接使用 rusqlite）：**

```rust
use rusqlite::{Connection, params};

fn get_users() -> Result<Vec<User>> {
    let conn = Connection::open("app.db")?;
    let mut stmt = conn.prepare("SELECT id, name FROM users")?;
    let rows = stmt.query_map([], |row| {
        Ok(User {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;

    let mut users = Vec::new();
    for user in rows {
        users.push(user?);
    }
    Ok(users)
}
```

**迁移后（使用 DataManager）：**

```rust
use crate::data::DataManager;

fn get_users() -> Result<Vec<User>> {
    let manager = DataManager::new();
    let db = manager.sqlite(Path::new("app.db"))?;  // 自动连接池

    // 查询结果自动缓存
    let rows = db.query("SELECT id, name FROM users", &[])?;

    let users = rows.into_iter().map(|row| {
        User {
            id: row.values[0].as_str().unwrap().to_string(),
            name: row.values[1].as_str().unwrap().to_string(),
        }
    }).collect();

    Ok(users)
}
```

## 🏗️ 架构设计

### 模块组织

```
src-tauri/src/data/
├── mod.rs              # 模块入口和文档
├── error.rs            # 统一错误类型
├── cache.rs            # LRU 缓存层
├── manager.rs          # DataManager 统一入口
└── managers/
    ├── mod.rs
    ├── json.rs         # JSON 管理器
    ├── toml.rs         # TOML 管理器
    ├── env.rs          # ENV 管理器
    └── sqlite.rs       # SQLite 管理器（连接池 + 查询缓存）
```

### 缓存机制

- **LRU 策略：** 默认缓存 100 个文件
- **失效条件：** 文件 mtime 改变时自动失效
- **校验和：** 基于文件内容的 SHA-256 校验
- **线程安全：** 使用 `Arc<Mutex<LruCache>>`

### 文件权限

- **Unix 系统：** 自动设置 0o600（仅所有者读写）
- **Windows：** 依赖系统默认权限
- **应用场景：** API Key、密码等敏感配置

### 原子写入

所有写操作使用临时文件 + rename 确保原子性：

```rust
// 1. 写入临时文件
let temp_path = path.with_extension("tmp");
fs::write(&temp_path, content)?;

// 2. 设置权限
#[cfg(unix)]
fs::set_permissions(&temp_path, perms)?;

// 3. 原子重命名
fs::rename(&temp_path, path)?;
```

## 📝 测试

项目包含完整的测试套件：

- **单元测试：** 16 个迁移测试（`data::migration_tests`）
- **集成测试：** 32 个配置管理测试
- **覆盖模块：** `utils/config.rs`、`services/config.rs`、`services/profile_store.rs`

运行测试：

```bash
# 运行所有数据管理相关测试
cargo test --lib data::

# 运行迁移测试
cargo test --lib data::migration_tests

# 运行配置服务测试
cargo test --lib services::config::tests
cargo test --lib services::profile_store::tests
```

## 🔍 故障排查

### 缓存未生效

**问题：** 修改文件后读取到旧数据

**解决：**

- 确认使用 `json()` 而非 `json_uncached()`
- 检查文件 mtime 是否正确更新
- 验证缓存大小限制（默认 100 个文件）

### TOML 注释丢失

**问题：** 保存 TOML 后注释消失

**解决：**

- 使用 `read_document()` 而非 `read()`
- 使用 `write(&DocumentMut)` 而非直接序列化

### 权限错误

**问题：** Unix 系统无法读取配置文件

**解决：**

- 检查文件权限：`ls -la config.json`
- 确认 DataManager 正确设置了 0o600
- 验证父目录权限

### SQLite 连接错误

**问题：** 数据库文件被锁定或无法打开

**解决：**

- 检查文件路径是否正确（使用绝对路径）
- 确认没有其他进程持有数据库锁
- 验证数据库文件权限（应为 0o600）
- 使用 `manager.sqlite()` 而非直接 `Connection::open()`

### SQLite 缓存不更新

**问题：** 查询结果未反映最新数据

**解决：**

- 确认写操作使用了 `execute()` 而非 `execute_raw()`
- 检查是否在事务外执行了直接写入
- 手动调用 `db.clear_cache()` 或 `db.invalidate_table("table_name")`

### SQLite 事务死锁

**问题：** 事务执行时超时或死锁

**解决：**

- 避免嵌套事务
- 减少事务持有时间
- 确保事务内的操作快速完成
- 检查是否有长时间运行的查询

## 📄 许可证

本项目采用 MIT 许可证。
