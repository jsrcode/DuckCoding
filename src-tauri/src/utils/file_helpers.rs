//! 文件操作辅助函数
//!
//! 提供常用的文件操作工具函数，如文件校验和计算等。

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// 计算文件的 SHA256 哈希值
///
/// 用于文件内容变更检测和完整性校验。
///
/// # 参数
///
/// * `path` - 文件路径
///
/// # 返回
///
/// * `Ok(String)` - 文件的 SHA256 哈希值（十六进制字符串）
/// * `Err` - 读取文件失败或计算哈希失败
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
/// use duckcoding::utils::file_helpers::file_checksum;
///
/// let checksum = file_checksum(Path::new("config.json"))?;
/// println!("文件校验和: {}", checksum);
/// ```
pub fn file_checksum(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};

    let content = fs::read(path).with_context(|| format!("读取文件失败: {path:?}"))?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let digest = hasher.finalize();
    Ok(format!("{digest:x}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_file_checksum() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(b"test content")?;
        temp_file.flush()?;

        let checksum = file_checksum(temp_file.path())?;

        // 验证返回的是64位十六进制字符串（SHA256）
        assert_eq!(checksum.len(), 64);
        assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));

        // 相同内容应该产生相同的校验和
        let checksum2 = file_checksum(temp_file.path())?;
        assert_eq!(checksum, checksum2);

        Ok(())
    }

    #[test]
    fn test_file_checksum_nonexistent() {
        let result = file_checksum(Path::new("/nonexistent/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_file_checksum_deterministic() -> Result<()> {
        let mut temp_file1 = NamedTempFile::new()?;
        let mut temp_file2 = NamedTempFile::new()?;

        temp_file1.write_all(b"same content")?;
        temp_file2.write_all(b"same content")?;
        temp_file1.flush()?;
        temp_file2.flush()?;

        let checksum1 = file_checksum(temp_file1.path())?;
        let checksum2 = file_checksum(temp_file2.path())?;

        // 相同内容的不同文件应该产生相同的校验和
        assert_eq!(checksum1, checksum2);

        Ok(())
    }
}
