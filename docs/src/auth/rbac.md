# RBAC 权限管理

> Feature: `rbac`

基于角色的访问控制（Role-Based Access Control）。通过 `RbacStore` trait 抽象存储层，用户自行实现具体存储逻辑（数据库、Redis、内存等）。

## 设计

- **RbacStore**：存储 trait，定义权限检查、角色管理等接口
- **Rbac**：对外 API，持有 `Arc<dyn RbacStore>`
- **数据模型**：`Role`、`UserRole` 等通用结构

## 使用

### 1. 实现 RbacStore trait

```rust
use afaster::rbac::{RbacStore, Role};
use afaster::Result;
use std::future::Future;
use std::pin::Pin;

struct MyRbacStore {
    pool: sqlx::SqlitePool,
}

impl RbacStore for MyRbacStore {
    fn check_permission(
        &self,
        user_id: i64,
        permission_code: &str,
    ) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + '_>> {
        Box::pin(async move {
            let count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM user_roles ur
                 JOIN role_permissions rp ON ur.role_id = rp.role_id
                 WHERE ur.user_id = ? AND rp.permission_code = ?",
            )
            .bind(user_id)
            .bind(permission_code)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| afaster::Error::custom(52803, format!("{}", e)))?;
            Ok(count.0 > 0)
        })
    }

    // ... 实现其他方法
}
```

### 2. 创建 Rbac 实例

```rust
let rbac = Rbac::new(MyRbacStore { pool });
```

### 3. 配合 AFaster 使用

```rust
AFaster::new("config.toml".to_string()).await
    .unwrap()
    .set_rbac(rbac)
    .run()
    .await;
```

### 4. 在 handler 中使用

```rust
#[handler]
async fn create_post(
    State(state): State<AppState>,
    Ctx(ctx): Ctx<AuthData>,
) -> Result<Json<Post>> {
    let user_id = ctx.user_id;
    let rbac = state.rbac.as_ref().unwrap();

    // 检查权限
    if !rbac.check_permission(user_id, "post:create").await? {
        return Err(afaster::Error::custom(42803, "权限不足"));
    }

    // ...
}
```

## RbacStore trait 方法

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `check_permission(user_id, code)` | `i64, &str` | `Result<bool>` | 检查用户是否有某权限 |
| `get_user_permissions(user_id)` | `i64` | `Result<Vec<String>>` | 获取用户所有权限码 |
| `get_user_roles(user_id)` | `i64` | `Result<Vec<Role>>` | 获取用户所有角色 |
| `assign_role(user_id, role_name)` | `i64, &str` | `Result<()>` | 分配角色 |
| `remove_role(user_id, role_name)` | `i64, &str` | `Result<()>` | 移除角色 |
| `create_role(name, display, perms)` | `&str, &str, &[&str]` | `Result<i64>` | 创建角色 |
| `delete_role(name)` | `&str` | `Result<()>` | 删除角色 |
| `get_role(name)` | `&str` | `Result<Role>` | 获取角色详情 |
| `list_roles()` | — | `Result<Vec<Role>>` | 列出所有角色 |
| `get_role_permissions(role_id)` | `i64` | `Result<Vec<String>>` | 获取角色的权限码 |
| `get_role_id(name)` | `&str` | `Result<i64>` | 获取角色 ID |

## 错误码

| 码 | English | 中文 |
|----|---------|------|
| 42801 | Role not found | 角色不存在 |
| 42802 | Cannot delete default role | 不能删除默认角色 |
| 42803 | Permission denied | 权限不足 |
| 52803 | Check failed | 权限检查失败 |
| 52804 | Assign failed | 角色分配失败 |
| 52805 | Role create failed | 角色创建失败 |
