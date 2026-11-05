use afaster::{orm::Orm, rbac::Rbac};
use afastorm::Field;

use crate::model::entities::{RbacRole, RbacRolePermission, User};

pub async fn initialize(orm: &Orm, rbac: &Rbac) {
    initialize_roles(orm).await;

    let users = orm.repo::<User>().expect("用户模型未注册");
    let admin_exists = users
        .first_where(Field::new("username").eq("admin"))
        .await
        .expect("查询默认管理员失败");

    if admin_exists.is_none() {
        let hash = afaster::argon2::Argon2Hasher::new()
            .hash("admin123")
            .expect("Hash failed");

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let mut admin = User {
            id: None,
            username: "admin".into(),
            nickname: "管理员".into(),
            email: "admin@blog.com".into(),
            password_hash: hash,
            avatar: String::new(),
            bio: String::new(),
            status: 0,
            created_at: now.clone(),
            updated_at: now,
        };
        users.insert(&mut admin).await.expect("Create admin failed");

        let admin_id = admin.id.expect("新建管理员未返回 ID");
        rbac.assign_role(admin_id, "super_admin")
            .await
            .expect("Assign role failed");
        println!("  ✅ 默认管理员: admin / admin123 (super_admin)");
    }
}

async fn initialize_roles(orm: &Orm) {
    let roles = vec![
        (
            "super_admin",
            "超级管理员",
            vec![
                "user:list",
                "user:read",
                "user:update",
                "user:delete",
                "user:ban",
                "role:list",
                "role:create",
                "role:update",
                "role:delete",
                "role:assign",
                "post:create",
                "post:update",
                "post:delete",
                "post:publish",
                "post:review",
                "comment:create",
                "comment:delete",
                "comment:review",
            ],
        ),
        (
            "reviewer",
            "审核人员",
            vec![
                "user:list",
                "user:read",
                "post:review",
                "comment:review",
                "comment:delete",
            ],
        ),
        ("user", "普通用户", vec!["post:create", "comment:create"]),
    ];

    for (name, display_name, permissions) in roles {
        let repository = orm.repo::<RbacRole>().expect("角色模型未注册");
        let role = repository
            .first_where(Field::new("name").eq(name))
            .await
            .expect("Failed to query role");
        let role_id = if let Some(role) = role {
            role.id.expect("角色记录没有主键")
        } else {
            let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let mut role = RbacRole {
                id: None,
                name: name.into(),
                display_name: display_name.into(),
                created_at: now,
            };
            repository
                .insert(&mut role)
                .await
                .expect("Failed to create role");
            role.id.expect("新建角色未返回 ID")
        };

        let permissions_repository = orm
            .repo::<RbacRolePermission>()
            .expect("角色权限模型未注册");
        for perm in permissions {
            let existing = permissions_repository
                .first_where(
                    Field::new("role_id")
                        .eq(role_id)
                        .and(Field::new("permission_code").eq(perm)),
                )
                .await
                .expect("Failed to query role permission");
            if existing.is_none() {
                let mut permission = RbacRolePermission {
                    role_id,
                    permission_code: (*perm).into(),
                };
                permissions_repository
                    .insert(&mut permission)
                    .await
                    .expect("Failed to create role permission");
            }
        }

        println!("  ✅ 角色: {} ({})", name, display_name);
    }
}
