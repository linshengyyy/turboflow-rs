use crate::types::{AuthInfo, Result};
use std::path::{Path, PathBuf};

/// 认证信息存储
#[derive(Debug, Clone)]
pub struct AuthStore {
    path: PathBuf,
}

impl Default for AuthStore {
    fn default() -> Self {
        Self::new("auth.json")
    }
}

impl AuthStore {
    /// 创建指定路径的存储
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// 获取当前路径
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 保存认证信息
    pub fn save(&self, auth: &AuthInfo) -> Result<()> {
        let content = serde_json::to_string_pretty(auth)?;
        std::fs::write(&self.path, content)?;
        Ok(())
    }

    /// 加载认证信息；文件不存在时返回默认空结构
    pub fn load(&self) -> Result<AuthInfo> {
        if !self.path.exists() {
            return Ok(AuthInfo::default());
        }
        let content = std::fs::read_to_string(&self.path)?;
        if content.trim().is_empty() {
            return Ok(AuthInfo::default());
        }
        Ok(serde_json::from_str(&content)?)
    }

    /// 清空认证信息（写入空 JSON 对象）
    pub fn clear(&self) -> Result<()> {
        std::fs::write(&self.path, "{}")?;
        Ok(())
    }
}

/// 便捷函数：保存到默认路径 `auth.json`
pub fn save_auth(token: &str, user_id: &str, email: &str) -> Result<()> {
    AuthStore::default().save(&AuthInfo {
        token: token.to_string(),
        user_id: user_id.to_string(),
        email: email.to_string(),
    })
}

/// 便捷函数：从默认路径 `auth.json` 加载
pub fn load_auth() -> Result<AuthInfo> {
    AuthStore::default().load()
}

/// 便捷函数：清空默认路径 `auth.json`
pub fn clear_auth() -> Result<()> {
    AuthStore::default().clear()
}
