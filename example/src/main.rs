//! 多人博客系统
//!
//! 三个 service：
//! - user: 公开接口（注册、登录、浏览文章）
//! - user_backend: 用户后台（管理自己的文章、评论）
//! - admin: 管理员接口（审核、用户管理、分类标签管理）

mod model;
mod service;

use afaster::Result;
use afaster::rbac::{Rbac, RbacStore, Role};
use afaster::{AFasterAcmeExt, AFasterRbacExt};
use std::future::Future;
use std::pin::Pin;

// ═══════════════════════════════════════════════════════════════
//  RBAC 存储实现（SQLite）
// ═══════════════════════════════════════════════════════════════

#[derive(Clone)]
struct SqliteRbacStore {
    pool: sqlx::sqlite::SqlitePool,
}

impl RbacStore for SqliteRbacStore {
    fn check_permission(
        &self,
        user_id: i64,
        permission_code: &str,
    ) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + '_>> {
        let permission_code = permission_code.to_string();
        Box::pin(async move {
            let count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM rbac_user_roles ur
                 JOIN rbac_role_permissions rp ON ur.role_id = rp.role_id
                 WHERE ur.user_id = ? AND rp.permission_code = ?",
            )
            .bind(user_id)
            .bind(&permission_code)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| afaster::Error::custom(52803, format!("{}", e)))?;
            Ok(count.0 > 0)
        })
    }

    fn get_user_permissions(
        &self,
        user_id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>>> + Send + '_>> {
        Box::pin(async move {
            let rows: Vec<(String,)> = sqlx::query_as(
                "SELECT DISTINCT rp.permission_code FROM rbac_user_roles ur
                 JOIN rbac_role_permissions rp ON ur.role_id = rp.role_id
                 WHERE ur.user_id = ?",
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| afaster::Error::custom(52803, format!("{}", e)))?;
            Ok(rows.into_iter().map(|r| r.0).collect())
        })
    }

    fn get_user_roles(
        &self,
        user_id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Role>>> + Send + '_>> {
        Box::pin(async move {
            let rows: Vec<(i64, String, String, String)> = sqlx::query_as(
                "SELECT r.id, r.name, r.display_name, r.created_at
                 FROM rbac_roles r
                 JOIN rbac_user_roles ur ON r.id = ur.role_id
                 WHERE ur.user_id = ?",
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| afaster::Error::custom(52803, format!("{}", e)))?;
            Ok(rows
                .into_iter()
                .map(|r| Role {
                    id: r.0,
                    name: r.1,
                    display_name: r.2,
                    created_at: r.3,
                })
                .collect())
        })
    }

    fn assign_role(
        &self,
        user_id: i64,
        role_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let role_name = role_name.to_string();
        Box::pin(async move {
            let role_id = self.get_role_id_inner(&role_name).await?;
            sqlx::query("INSERT OR IGNORE INTO rbac_user_roles (user_id, role_id) VALUES (?, ?)")
                .bind(user_id)
                .bind(role_id)
                .execute(&self.pool)
                .await
                .map_err(|e| afaster::Error::custom(52804, format!("{}", e)))?;
            Ok(())
        })
    }

    fn remove_role(
        &self,
        user_id: i64,
        role_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let role_name = role_name.to_string();
        Box::pin(async move {
            let role_id = self.get_role_id_inner(&role_name).await?;
            sqlx::query("DELETE FROM rbac_user_roles WHERE user_id = ? AND role_id = ?")
                .bind(user_id)
                .bind(role_id)
                .execute(&self.pool)
                .await
                .map_err(|e| afaster::Error::custom(52804, format!("{}", e)))?;
            Ok(())
        })
    }

    fn create_role(
        &self,
        name: &str,
        display_name: &str,
        permissions: &[&str],
    ) -> Pin<Box<dyn Future<Output = Result<i64>> + Send + '_>> {
        let name = name.to_string();
        let display_name = display_name.to_string();
        let permissions: Vec<String> = permissions.iter().map(|s| s.to_string()).collect();
        Box::pin(async move {
            sqlx::query("INSERT INTO rbac_roles (name, display_name) VALUES (?, ?)")
                .bind(&name)
                .bind(&display_name)
                .execute(&self.pool)
                .await
                .map_err(|e| afaster::Error::custom(52805, format!("{}", e)))?;

            let role_id = self.get_role_id_inner(&name).await?;

            for perm_code in &permissions {
                sqlx::query(
                    "INSERT OR IGNORE INTO rbac_role_permissions (role_id, permission_code) VALUES (?, ?)"
                )
                .bind(role_id)
                .bind(perm_code)
                .execute(&self.pool)
                .await
                .map_err(|e| afaster::Error::custom(52805, format!("{}", e)))?;
            }

            Ok(role_id)
        })
    }

    fn delete_role(&self, name: &str) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let name = name.to_string();
        Box::pin(async move {
            sqlx::query("DELETE FROM rbac_roles WHERE name = ?")
                .bind(&name)
                .execute(&self.pool)
                .await
                .map_err(|e| afaster::Error::custom(52805, format!("{}", e)))?;
            Ok(())
        })
    }

    fn get_role(&self, name: &str) -> Pin<Box<dyn Future<Output = Result<Role>> + Send + '_>> {
        let name = name.to_string();
        Box::pin(async move {
            let row: (i64, String, String, String) = sqlx::query_as(
                "SELECT id, name, display_name, created_at FROM rbac_roles WHERE name = ?",
            )
            .bind(&name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| afaster::Error::custom(42801, format!("{}: {}", name, e)))?;
            Ok(Role {
                id: row.0,
                name: row.1,
                display_name: row.2,
                created_at: row.3,
            })
        })
    }

    fn list_roles(&self) -> Pin<Box<dyn Future<Output = Result<Vec<Role>>> + Send + '_>> {
        Box::pin(async move {
            let rows: Vec<(i64, String, String, String)> = sqlx::query_as(
                "SELECT id, name, display_name, created_at FROM rbac_roles ORDER BY id ASC",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| afaster::Error::custom(52803, format!("{}", e)))?;
            Ok(rows
                .into_iter()
                .map(|r| Role {
                    id: r.0,
                    name: r.1,
                    display_name: r.2,
                    created_at: r.3,
                })
                .collect())
        })
    }

    fn get_role_permissions(
        &self,
        role_id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>>> + Send + '_>> {
        Box::pin(async move {
            let rows: Vec<(String,)> = sqlx::query_as(
                "SELECT permission_code FROM rbac_role_permissions WHERE role_id = ?",
            )
            .bind(role_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| afaster::Error::custom(52803, format!("{}", e)))?;
            Ok(rows.into_iter().map(|r| r.0).collect())
        })
    }

    fn get_role_id(&self, name: &str) -> Pin<Box<dyn Future<Output = Result<i64>> + Send + '_>> {
        let name = name.to_string();
        Box::pin(async move { self.get_role_id_inner(&name).await })
    }
}

impl SqliteRbacStore {
    async fn get_role_id_inner(&self, name: &str) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT id FROM rbac_roles WHERE name = ?")
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| afaster::Error::custom(42801, format!("{}: {}", name, e)))?;
        Ok(row.0)
    }
}

// ═══════════════════════════════════════════════════════════════
//  主函数
// ═══════════════════════════════════════════════════════════════

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║              多人博客系统 (Blog System)                  ║");
    println!("║    HTTP Binary + RBAC + SQLite + JWT + Argon2 ..         ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    // ── 初始化数据库 ──────────────────────────────────────────
    let db_path = "blog.db";
    let pool = sqlx::sqlite::SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path))
        .await
        .expect("数据库连接失败");

    init_database(&pool).await;

    // ── 初始化 RBAC ──────────────────────────────────────────
    let rbac_store = SqliteRbacStore { pool: pool.clone() };

    // 创建 RBAC 表
    init_rbac_tables(&pool).await;

    // 初始化默认角色和权限
    init_rbac_data(&pool).await;

    let rbac = Rbac::new(rbac_store);

    // 创建默认管理员
    let admin_exists: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE username = 'admin'")
            .fetch_one(&pool)
            .await
            .unwrap_or((0,));

    if admin_exists.0 == 0 {
        let hash = afaster::argon2::Argon2Hasher::new()
            .hash("admin123")
            .expect("Hash failed");

        sqlx::query(
            "INSERT INTO users (username, nickname, email, password_hash) VALUES (?, ?, ?, ?)",
        )
        .bind("admin")
        .bind("管理员")
        .bind("admin@blog.com")
        .bind(&hash)
        .execute(&pool)
        .await
        .expect("Create admin failed");

        let admin_id: (i64,) = sqlx::query_as("SELECT id FROM users WHERE username = 'admin'")
            .fetch_one(&pool)
            .await
            .unwrap();
        rbac.assign_role(admin_id.0, "super_admin")
            .await
            .expect("Assign role failed");
        println!("  ✅ 默认管理员: admin / admin123 (super_admin)");
    }

    pool.close().await;

    // ── 启动 AFaster ─────────────────────────────────────────
    let app = afaster::AFaster::new("example/config.toml".to_string())
        .await
        .expect("初始化失败")
        .set_rbac(rbac)
        .doc_title("Blog API Documentation")
        .generate(afast::GenerateTarget {
            lang: afast::Lang::TS(vec![afast::JsTsCallType::Fetch, afast::JsTsCallType::Ws]),
            path: std::path::PathBuf::from("example/client/ts"),
            debug: true,
            services: None,
        })
        .generate(afast::GenerateTarget {
            lang: afast::Lang::JS(vec![afast::JsTsCallType::Fetch, afast::JsTsCallType::Ws]),
            path: std::path::PathBuf::from("example/client/js"),
            debug: true,
            services: None,
        })
        .generate(afast::GenerateTarget {
            lang: afast::Lang::KT(vec![afast::KtCallType::Tcp]),
            path: std::path::PathBuf::from("example/client/kt"),
            debug: true,
            services: None,
        })
        .generate(afast::GenerateTarget {
            lang: afast::Lang::RS(vec![afast::RsCallType::TcpAsync]),
            path: std::path::PathBuf::from("example/client/rs"),
            debug: true,
            services: None,
        })
        .service(service::user::build_service())
        .service(service::user_backend::build_service())
        .service(service::admin::build_service())
        .service(service::bin_chat::build_service())
        .service(service::ws_chat::build_service("/ws/chat"))
        .service(service::http_api::build_service())
        .with_acme(|acme| {
            acme.with_on_cert_obtained(
                |(state, _event): (afaster::AppState, afaster::acme::CertEvent)| async move {
                    state.tls.reload();
                    Ok(())
                },
            )
            .with_on_cert_renewed(
                |(state, _event): (afaster::AppState, afaster::acme::CertEvent)| async move {
                    state.tls.reload();
                    Ok(())
                },
            )
            .with_on_cert_failed(
                |(_state, event): (afaster::AppState, afaster::acme::CertErrorEvent)| async move {
                    eprintln!("ACME 证书失败: {:?} - {}", event.domains, event.error);
                    Ok(())
                },
            )
        });

    println!("  🚀 博客系统启动中...");
    println!("  📡 HTTP Binary 端口: 8080");
    println!("  📋 API 列表:");
    println!(
        "     [user]          register, login, list_posts, get_post, list_categories, list_tags"
    );
    println!(
        "     [user_backend]  me, create_post, my_posts, delete_post, create_comment, like_post"
    );
    println!("     [admin]         ban_user, unban_user, assign_role, review_post, pending_posts,");
    println!("                     create_category, delete_category, create_tag, delete_tag,");
    println!("                     delete_comment, list_roles, list_permissions");
    println!("     [ws_chat]       ws_chat (WS /ws/chat)");
    println!("     [bin_chat]      bin_chat_echo, bin_notify (Binary)");
    println!("     [api]           GET /api/categories, /api/tags, /api/posts, /api/stats");
    println!("                     POST /api/posts");

    app.run().await;
}

// ═══════════════════════════════════════════════════════════════
//  数据库初始化
// ═══════════════════════════════════════════════════════════════

async fn init_database(pool: &sqlx::sqlite::SqlitePool) {
    let tables = vec![
        (
            "users",
            "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            nickname TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            avatar TEXT NOT NULL DEFAULT '',
            bio TEXT NOT NULL DEFAULT '',
            status INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        ),
        (
            "categories",
            "CREATE TABLE IF NOT EXISTS categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            slug TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL DEFAULT '',
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        ),
        (
            "tags",
            "CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            slug TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        ),
        (
            "posts",
            "CREATE TABLE IF NOT EXISTS posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            author_id INTEGER NOT NULL,
            category_id INTEGER,
            title TEXT NOT NULL,
            slug TEXT NOT NULL,
            summary TEXT NOT NULL DEFAULT '',
            content TEXT NOT NULL,
            cover_image TEXT NOT NULL DEFAULT '',
            status INTEGER NOT NULL DEFAULT 0,
            view_count INTEGER NOT NULL DEFAULT 0,
            like_count INTEGER NOT NULL DEFAULT 0,
            is_pinned BOOLEAN NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            published_at TEXT,
            FOREIGN KEY (author_id) REFERENCES users(id),
            FOREIGN KEY (category_id) REFERENCES categories(id)
        )",
        ),
        (
            "post_tags",
            "CREATE TABLE IF NOT EXISTS post_tags (
            post_id INTEGER NOT NULL,
            tag_id INTEGER NOT NULL,
            PRIMARY KEY (post_id, tag_id),
            FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
        )",
        ),
        (
            "post_likes",
            "CREATE TABLE IF NOT EXISTS post_likes (
            post_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (post_id, user_id),
            FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
        ),
        (
            "comments",
            "CREATE TABLE IF NOT EXISTS comments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            post_id INTEGER NOT NULL,
            author_id INTEGER NOT NULL,
            parent_id INTEGER,
            content TEXT NOT NULL,
            status INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (post_id) REFERENCES posts(id),
            FOREIGN KEY (author_id) REFERENCES users(id),
            FOREIGN KEY (parent_id) REFERENCES comments(id)
        )",
        ),
    ];

    for (name, sql) in tables {
        match sqlx::query(sql).execute(pool).await {
            Ok(_) => println!("  ✅ 建表: {}", name),
            Err(e) => println!("  ❌ 建表 {} 失败: {}", name, e),
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  RBAC 表初始化
// ═══════════════════════════════════════════════════════════════

async fn init_rbac_tables(pool: &sqlx::sqlite::SqlitePool) {
    let tables = vec![
        (
            "rbac_roles",
            "CREATE TABLE IF NOT EXISTS rbac_roles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        ),
        (
            "rbac_role_permissions",
            "CREATE TABLE IF NOT EXISTS rbac_role_permissions (
            role_id INTEGER NOT NULL,
            permission_code TEXT NOT NULL,
            PRIMARY KEY (role_id, permission_code),
            FOREIGN KEY (role_id) REFERENCES rbac_roles(id) ON DELETE CASCADE
        )",
        ),
        (
            "rbac_user_roles",
            "CREATE TABLE IF NOT EXISTS rbac_user_roles (
            user_id INTEGER NOT NULL,
            role_id INTEGER NOT NULL,
            PRIMARY KEY (user_id, role_id),
            FOREIGN KEY (role_id) REFERENCES rbac_roles(id) ON DELETE CASCADE
        )",
        ),
    ];

    for (name, sql) in tables {
        match sqlx::query(sql).execute(pool).await {
            Ok(_) => println!("  ✅ 建表: {}", name),
            Err(e) => println!("  ❌ 建表 {} 失败: {}", name, e),
        }
    }
}

async fn init_rbac_data(pool: &sqlx::sqlite::SqlitePool) {
    // 定义角色和权限
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
        // 插入角色
        sqlx::query("INSERT OR IGNORE INTO rbac_roles (name, display_name) VALUES (?, ?)")
            .bind(name)
            .bind(display_name)
            .execute(pool)
            .await
            .expect("Failed to create role");

        // 获取角色 ID
        let role_id: (i64,) = sqlx::query_as("SELECT id FROM rbac_roles WHERE name = ?")
            .bind(name)
            .fetch_one(pool)
            .await
            .expect("Failed to get role id");

        // 插入权限关联
        for perm in permissions {
            sqlx::query(
                "INSERT OR IGNORE INTO rbac_role_permissions (role_id, permission_code) VALUES (?, ?)"
            )
            .bind(role_id.0)
            .bind(perm)
            .execute(pool)
            .await
            .expect("Failed to create role permission");
        }

        println!("  ✅ 角色: {} ({})", name, display_name);
    }
}
