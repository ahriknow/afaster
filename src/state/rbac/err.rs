//! RBAC 错误码
//!
//! 模块编号：28（428xx / 528xx）

use crate::Error;

/// 角色不存在
pub fn role_not_found(detail: &str) -> Error {
    Error::custom(42801, format!("Role not found: {}", detail))
}

/// 不能删除默认角色
pub fn cannot_delete_default_role() -> Error {
    Error::custom(42802, "Cannot delete default role".to_string())
}

/// 权限不足
pub fn permission_denied(detail: &str) -> Error {
    Error::custom(42803, format!("Permission denied: {}", detail))
}

/// 权限检查失败
pub fn check_failed(detail: &str) -> Error {
    Error::custom(52803, format!("RBAC check failed: {}", detail))
}

/// 角色分配失败
pub fn assign_failed(detail: &str) -> Error {
    Error::custom(52804, format!("RBAC assign failed: {}", detail))
}

/// 角色创建失败
pub fn role_create_failed(detail: &str) -> Error {
    Error::custom(52805, format!("RBAC role create failed: {}", detail))
}
