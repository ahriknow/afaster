use afaster::{AFasterOrmExt, AFasterRbacExt};
use std::collections::HashMap;

use crate::{bootstrap, model, rbac::SqliteRbacStore, service};

pub async fn run() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║              多人博客系统 (Blog System)                  ║");
    println!("║    HTTP Binary + RBAC + SQLite + JWT + Argon2 ..         ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let app = afaster::AFaster::new("example/config.toml".to_string())
        .await
        .expect("初始化失败")
        .with_orm(|orm| {
            orm.with_models(model::entities::registry())
                .with_admin(true)
                .with_admin_logo_letter("B")
                .with_admin_brand_text("博客管理")
                .with_admin_models(model::entities::admin_registry())
                .with_i18ns(vec![
                    (
                        "zh-CN".to_owned(),
                        HashMap::from([
                            ("model.category".to_owned(), "分类".to_owned()),
                            ("model.tag".to_owned(), "标签".to_owned()),
                            ("model.post".to_owned(), "文章".to_owned()),
                            ("model.comment".to_owned(), "评论".to_owned()),
                            ("category.name".to_owned(), "名称".to_owned()),
                            ("tag.name".to_owned(), "名称".to_owned()),
                            ("post.title".to_owned(), "标题".to_owned()),
                            ("post.published_at".to_owned(), "发布时间".to_owned()),
                            ("comment.content".to_owned(), "内容".to_owned()),
                        ]),
                    ),
                    (
                        "en".to_owned(),
                        HashMap::from([
                            ("model.category".to_owned(), "Categories".to_owned()),
                            ("model.tag".to_owned(), "Tags".to_owned()),
                            ("model.post".to_owned(), "Posts".to_owned()),
                            ("model.comment".to_owned(), "Comments".to_owned()),
                            ("category.name".to_owned(), "Name".to_owned()),
                            ("tag.name".to_owned(), "Name".to_owned()),
                            ("post.title".to_owned(), "Title".to_owned()),
                            ("post.published_at".to_owned(), "Published at".to_owned()),
                            ("comment.content".to_owned(), "Content".to_owned()),
                        ]),
                    ),
                ])
        })
        .expect("ORM 配置失败");
    match app.run_command_if_requested().await {
        Ok(true) => return,
        Ok(false) => {}
        Err(error) => {
            eprintln!("AFaster 命令执行失败: {error}");
            std::process::exit(2);
        }
    }

    let orm = app.state().orm.clone();
    let app = app
        .with_rbac(|rbac| rbac.with_store(SqliteRbacStore::new(orm.clone())))
        .expect("RBAC 配置失败");
    let rbac = app.state().rbac.as_ref().expect("RBAC 模块未注册").clone();
    bootstrap::initialize(&orm, &rbac).await;

    let app = app
        .with_cors(afast::CorsConfig::permissive())
        .doc_title("Blog API Documentation")
        .doc_basic_auth("username", "password")
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
        .service(service::http_api::build_service());

    let admin_url = app.state().orm.admin_url().unwrap_or("/admin");
    println!(
        "  🛠️ ORM 管理端: http://{}:{}{}",
        app.state().backend.host,
        app.state().backend.port,
        admin_url
    );
    println!("  🚀 博客系统启动中...");
    println!("  📡 HTTP Binary 端口: {}", app.state().backend.port);
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

#[cfg(test)]
mod tests {
    use afastorm::{Field, Statement, Value};

    use crate::model::entities::{
        self, Post, PostFindBy as _, PostFindJson as _, User, UserFindBy as _,
    };

    #[test]
    fn admin_creation_metadata_exposes_required_fields() {
        fn assert_create_fields<M: afaster::orm::AFasterAdmin>(required: &[&str]) {
            for name in required {
                assert!(
                    M::ADMIN_INFO
                        .fields
                        .iter()
                        .any(|field| field.rust_name == *name && field.create),
                    "{} must be available when creating {}",
                    name,
                    M::ADMIN_INFO.rust_name
                );
            }
        }

        assert_create_fields::<crate::model::entities::Category>(&["name", "slug"]);
        assert_create_fields::<crate::model::entities::Tag>(&["name", "slug"]);
        assert_create_fields::<Post>(&["author_id", "title", "slug", "content"]);
        assert_create_fields::<crate::model::entities::Comment>(&[
            "post_id",
            "author_id",
            "content",
        ]);
    }

    #[tokio::test]
    async fn blog_models_register_and_persist_through_repositories() {
        let orm = afastorm::Orm::connect("sqlite::memory:")
            .await
            .expect("connect test database")
            .with_models(entities::registry());
        orm.create_registered_tables()
            .await
            .expect("create registered model tables");

        let now = chrono::NaiveDateTime::parse_from_str("2026-11-02 00:00:00", "%Y-%m-%d %H:%M:%S")
            .expect("valid test timestamp");
        let mut user = User {
            id: None,
            username: "test-user".into(),
            nickname: "Test User".into(),
            email: "test@example.com".into(),
            password_hash: "hash".into(),
            avatar: String::new(),
            bio: String::new(),
            status: 0,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let mut post = Post {
            id: None,
            author_id: 0,
            category_id: None,
            title: "Typed ORM".into(),
            slug: "typed-orm".into(),
            summary: "Model-backed blog query".into(),
            content: "Body".into(),
            cover_image: String::new(),
            metadata: serde_json::json!({
                "featured": true,
                "topics": ["orm", "rust"],
            }),
            status: 2,
            view_count: 0,
            like_count: 0,
            is_pinned: true,
            created_at: now,
            updated_at: now,
            published_at: Some(now),
            deleted_at: None,
            version: 0,
        };

        let post_id = orm
            .transaction(async |transaction| {
                transaction.repo::<User>()?.insert(&mut user).await?;
                let user_id = user.id.ok_or_else(|| {
                    afastorm::Error::Query("user ID was not generated".to_owned())
                })?;
                post.author_id = user_id;
                transaction.repo::<Post>()?.insert(&mut post).await?;
                post.id
                    .ok_or_else(|| afastorm::Error::Query("post ID was not generated".to_owned()))
            })
            .await
            .expect("commit user and post together");
        let user_id = user.id.expect("generated user id");
        assert_eq!(post_id, post.id.expect("generated post id"));

        let posts = orm.repo::<Post>().expect("post repository");
        assert_eq!(
            posts.count_where(Field::new("status").eq(2)).await.unwrap(),
            1
        );
        let found_user = orm
            .repo::<User>()
            .expect("user repository")
            .find_by_username("test-user".to_owned())
            .await
            .expect("generated find_by user");
        assert_eq!(found_user.expect("user exists").id, Some(user_id));

        let fetched = posts
            .find_by_author_id_slug(user_id, "typed-orm".to_owned())
            .await
            .expect("generated composite find_by")
            .expect("post exists");
        assert_eq!(fetched.title, "Typed ORM");
        assert!(fetched.is_pinned);
        assert_eq!(fetched.version, 1);
        assert_ne!(fetched.created_at, chrono::NaiveDateTime::default());
        assert_eq!(fetched.published_at, Some(now));

        let featured = posts
            .find_json_eq_metadata_featured(serde_json::json!(true))
            .await
            .expect("generated JSON lookup")
            .expect("featured post");
        assert_eq!(featured.id, Some(post_id));
        assert!(
            posts
                .find_json_contains_metadata(serde_json::json!({"topics": ["orm"]}))
                .await
                .expect("JSON containment lookup")
                .is_some()
        );

        posts
            .delete_versioned(post_id, fetched.version)
            .await
            .expect("versioned soft delete");
        let deleted_post_query = Statement::from_sql(
            orm.dialect(),
            "SELECT * FROM posts WHERE id = ?",
            vec![Value::I64(post_id)],
        )
        .expect("build deleted post query");
        let deleted_post = posts
            .fetch_all(&deleted_post_query)
            .await
            .expect("decode soft-deleted post")
            .pop()
            .expect("soft-deleted post exists");
        assert!(deleted_post.deleted_at.is_some());
        assert!(posts.find_by_id(post_id).await.unwrap().is_none());
        posts
            .restore_versioned(post_id, fetched.version + 1)
            .await
            .expect("versioned restore");
        assert_eq!(
            posts
                .find_by_id(post_id)
                .await
                .unwrap()
                .expect("restored post")
                .version,
            fetched.version + 2
        );

        let mut rollback_probe = Post {
            id: None,
            author_id: user_id,
            category_id: None,
            title: "Rolled back".into(),
            slug: "rolled-back".into(),
            summary: String::new(),
            content: "This row must not persist".into(),
            cover_image: String::new(),
            metadata: serde_json::json!({}),
            status: 1,
            view_count: 0,
            like_count: 0,
            is_pinned: false,
            created_at: chrono::NaiveDateTime::default(),
            updated_at: chrono::NaiveDateTime::default(),
            published_at: None,
            deleted_at: None,
            version: 0,
        };
        let result = orm
            .transaction(async |transaction| {
                transaction
                    .repo::<Post>()?
                    .insert(&mut rollback_probe)
                    .await?;
                Err::<(), _>(afastorm::Error::Query("rollback test".to_owned()))
            })
            .await;
        assert!(result.is_err());
        assert!(
            posts
                .find_by_slug("rolled-back".to_owned())
                .await
                .unwrap()
                .is_none()
        );
    }
}
