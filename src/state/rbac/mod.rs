//! RBAC 权限管理
//!
//! 基于角色的访问控制（Role-Based Access Control）。
//! 通过 `RbacStore` trait 抽象存储层，用户自行实现具体存储逻辑。
//!
//! # 设计
//!
//! - **RbacStore**：存储 trait，定义权限检查、角色管理等接口
//! - **Rbac**：对外 API，持有 `Arc<dyn RbacStore>`
//! - **数据模型**：`Role`、`UserRole` 等通用结构
//!
//! # 使用
//!
//! ```ignore
//! // 1. 实现 RbacStore trait
//! struct MyRbacStore { /* 数据库连接等 */ }
//!
//! impl RbacStore for MyRbacStore {
//!     async fn check_permission(&self, user_id: i64, code: &str) -> Result<bool> {
//!         // 查询数据库...
//!     }
//!     // ... 其他方法
//! }
//!
//! // 2. 创建 Rbac 实例
//! let rbac = Rbac::new(MyRbacStore { ... });
//!
//! // 3. 使用
//! let has = rbac.check_permission(user_id, "post:create").await?;
//! rbac.assign_role(user_id, "editor").await?;
//! ```

pub mod err;

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// ═══════════════════════════════════════════════════════════════
//  数据模型
// ═══════════════════════════════════════════════════════════════

/// 角色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub created_at: String,
}

/// 用户角色关联
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRole {
    pub user_id: i64,
    pub role_id: i64,
}

// ═══════════════════════════════════════════════════════════════
//  RbacStore trait
// ═══════════════════════════════════════════════════════════════

/// RBAC 存储 trait
///
/// 用户实现此 trait 来提供具体的权限存储逻辑（数据库、Redis、内存等）。
/// 使用 `Pin<Box<dyn Future>>` 以支持动态分发。
pub trait RbacStore: Send + Sync + 'static {
    /// 检查用户是否有某权限
    fn check_permission(
        &self,
        user_id: i64,
        permission_code: &str,
    ) -> Pin<Box<dyn Future<Output = crate::Result<bool>> + Send + '_>>;

    /// 获取用户的所有权限码
    fn get_user_permissions(
        &self,
        user_id: i64,
    ) -> Pin<Box<dyn Future<Output = crate::Result<Vec<String>>> + Send + '_>>;

    /// 获取用户的所有角色
    fn get_user_roles(
        &self,
        user_id: i64,
    ) -> Pin<Box<dyn Future<Output = crate::Result<Vec<Role>>> + Send + '_>>;

    /// 分配角色给用户
    fn assign_role(
        &self,
        user_id: i64,
        role_name: &str,
    ) -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send + '_>>;

    /// 移除用户角色
    fn remove_role(
        &self,
        user_id: i64,
        role_name: &str,
    ) -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send + '_>>;

    /// 创建角色，返回角色 ID
    fn create_role(
        &self,
        name: &str,
        display_name: &str,
        permissions: &[&str],
    ) -> Pin<Box<dyn Future<Output = crate::Result<i64>> + Send + '_>>;

    /// 删除角色
    fn delete_role(
        &self,
        name: &str,
    ) -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send + '_>>;

    /// 获取角色详情
    fn get_role(
        &self,
        name: &str,
    ) -> Pin<Box<dyn Future<Output = crate::Result<Role>> + Send + '_>>;

    /// 列出所有角色
    fn list_roles(&self) -> Pin<Box<dyn Future<Output = crate::Result<Vec<Role>>> + Send + '_>>;

    /// 获取角色的权限码列表
    fn get_role_permissions(
        &self,
        role_id: i64,
    ) -> Pin<Box<dyn Future<Output = crate::Result<Vec<String>>> + Send + '_>>;

    /// 获取角色 ID
    fn get_role_id(
        &self,
        name: &str,
    ) -> Pin<Box<dyn Future<Output = crate::Result<i64>> + Send + '_>>;
}

// ═══════════════════════════════════════════════════════════════
//  RBAC 模块
// ═══════════════════════════════════════════════════════════════

/// RBAC 权限管理模块
#[derive(Clone)]
pub struct Rbac {
    store: Arc<dyn RbacStore>,
}

impl Rbac {
    /// 创建 RBAC 实例
    pub fn new(store: impl RbacStore) -> Self {
        Self {
            store: Arc::new(store),
        }
    }

    /// 从已有的 Arc 创建 RBAC 实例
    pub fn from_arc(store: Arc<dyn RbacStore>) -> Self {
        Self { store }
    }

    /// 获取内部 store 引用
    pub fn store(&self) -> &dyn RbacStore {
        self.store.as_ref()
    }

    /// 检查用户是否有某权限
    pub async fn check_permission(
        &self,
        user_id: i64,
        permission_code: &str,
    ) -> crate::Result<bool> {
        self.store.check_permission(user_id, permission_code).await
    }

    /// 获取用户的所有权限码
    pub async fn get_user_permissions(&self, user_id: i64) -> crate::Result<Vec<String>> {
        self.store.get_user_permissions(user_id).await
    }

    /// 获取用户的所有角色
    pub async fn get_user_roles(&self, user_id: i64) -> crate::Result<Vec<Role>> {
        self.store.get_user_roles(user_id).await
    }

    /// 分配角色给用户
    pub async fn assign_role(&self, user_id: i64, role_name: &str) -> crate::Result<()> {
        self.store.assign_role(user_id, role_name).await
    }

    /// 移除用户角色
    pub async fn remove_role(&self, user_id: i64, role_name: &str) -> crate::Result<()> {
        self.store.remove_role(user_id, role_name).await
    }

    /// 创建角色
    pub async fn create_role(
        &self,
        name: &str,
        display_name: &str,
        permissions: &[&str],
    ) -> crate::Result<i64> {
        self.store
            .create_role(name, display_name, permissions)
            .await
    }

    /// 删除角色
    pub async fn delete_role(&self, name: &str) -> crate::Result<()> {
        self.store.delete_role(name).await
    }

    /// 获取角色详情
    pub async fn get_role(&self, name: &str) -> crate::Result<Role> {
        self.store.get_role(name).await
    }

    /// 列出所有角色
    pub async fn list_roles(&self) -> crate::Result<Vec<Role>> {
        self.store.list_roles().await
    }

    /// 获取角色的权限码列表
    pub async fn get_role_permissions(&self, role_id: i64) -> crate::Result<Vec<String>> {
        self.store.get_role_permissions(role_id).await
    }

    /// 获取角色 ID
    pub async fn get_role_id(&self, name: &str) -> crate::Result<i64> {
        self.store.get_role_id(name).await
    }
}
