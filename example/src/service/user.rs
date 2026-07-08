//! 用户公开接口：注册、登录、浏览文章、分类、标签

use crate::model::*;
use afast::{Ctx, Data, State, handler};
use afaster::trace::TraceContext;

// ── 注册 ──────────────────────────────────────────────────────

#[handler(desc("用户注册"))]
async fn register(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Data(req): Data<RegisterReq>,
) -> afast::Result<ApiResp> {
    let pool = state.db.sqlite();

    let exists: (i64,) = {
        let _s = state
            .tracing
            .span_with(&trace, "check_username", "检查用户名");
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE username = ?")
            .bind(&req.username)
            .fetch_one(pool)
            .await
            .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))?
    };
    if exists.0 > 0 {
        return Ok(ApiResp::err(40001, "用户名已存在"));
    }

    let hash = {
        let _s = state.tracing.span_with(&trace, "hash_password", "哈希密码");
        afaster::argon2::Argon2Hasher::new()
            .hash(&req.password)
            .map_err(|e| afaster::Error::custom(52402, format!("Hash: {}", e)))?
    };

    {
        let _s = state.tracing.span_with(&trace, "insert_user", "写入用户");
        sqlx::query(
            "INSERT INTO users (username, nickname, email, password_hash) VALUES (?, ?, ?, ?)",
        )
        .bind(&req.username)
        .bind(&req.nickname)
        .bind(&req.email)
        .bind(&hash)
        .execute(pool)
        .await
        .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))?;
    }

    let uid: (i64,) = {
        let _s = state
            .tracing
            .span_with(&trace, "query_user_id", "查询用户ID");
        sqlx::query_as("SELECT id FROM users WHERE username = ?")
            .bind(&req.username)
            .fetch_one(pool)
            .await
            .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))?
    };

    {
        let _s = state.tracing.span_with(&trace, "assign_role", "分配角色");
        if let Some(ref rbac) = state.rbac {
            let _ = rbac.assign_role(uid.0, "user").await;
        }
    }
    Ok(ApiResp::ok("注册成功"))
}

// ── 登录 ──────────────────────────────────────────────────────

#[handler(desc("用户登录"))]
async fn login(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Data(req): Data<LoginReq>,
) -> afast::Result<LoginResp> {
    let pool = state.db.sqlite();

    let user: (i64, String, String, String, String, String, String, i32) = {
        let _s = state.tracing.span_with(&trace, "db_query_user", "查询用户");
        sqlx::query_as(
            "SELECT id, username, nickname, email, password_hash, avatar, bio, status FROM users WHERE username = ?"
        ).bind(&req.username).fetch_one(pool).await
            .map_err(|_| afaster::Error::custom(40001, "用户名或密码错误"))?
    };
    if user.7 != 0 {
        return Ok(LoginResp {
            code: 40003,
            message: "账号已被封禁".into(),
            token: String::new(),
            user_id: 0,
            username: String::new(),
            nickname: String::new(),
            roles: vec![],
            permissions: vec![],
        });
    }

    {
        let _s = state
            .tracing
            .span_with(&trace, "verify_password", "验证密码");
        let valid = afaster::argon2::Argon2Hasher::new()
            .verify(&req.password, &user.4)
            .map_err(|e| afaster::Error::custom(52402, format!("Verify: {}", e)))?;
        if !valid {
            return Ok(LoginResp {
                code: 40001,
                message: "用户名或密码错误".into(),
                token: String::new(),
                user_id: 0,
                username: String::new(),
                nickname: String::new(),
                roles: vec![],
                permissions: vec![],
            });
        }
    }

    let (roles, permissions) = {
        let _s = state
            .tracing
            .span_with(&trace, "query_permissions", "查询权限");
        if let Some(ref rbac) = state.rbac {
            let r = rbac.get_user_roles(user.0).await.unwrap_or_default();
            let p = rbac.get_user_permissions(user.0).await.unwrap_or_default();
            (r.iter().map(|x| x.name.clone()).collect(), p)
        } else {
            (vec![], vec![])
        }
    };

    let token = {
        let _s = state
            .tracing
            .span_with(&trace, "create_token", "生成 Token");
        state
            .token
            .create_token(
                user.0,
                afaster::token::Data1 {
                    id: user.0,
                    username: user.1.clone(),
                    nickname: user.2.clone(),
                    avatar: user.5.clone(),
                    is_super: roles.contains(&"super_admin".to_string()),
                },
            )
            .map_err(|e| afaster::Error::custom(50101, format!("JWT: {}", e)))?
    };

    Ok(LoginResp {
        code: 0,
        message: "登录成功".into(),
        token,
        user_id: user.0,
        username: user.1,
        nickname: user.2,
        roles,
        permissions,
    })
}

// ── 文章列表 ──────────────────────────────────────────────────

#[handler(desc("文章列表"))]
async fn list_posts(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Data(req): Data<ListPostsReq>,
) -> afast::Result<PostListResp> {
    let pool = state.db.sqlite();
    let page = req.page.max(1);
    let page_size = req.page_size.clamp(1, 100);
    let offset = (page - 1) * page_size;

    let total: (i64,) = {
        let _s = state.tracing.span_with(&trace, "count_posts", "统计文章");
        sqlx::query_as("SELECT COUNT(*) FROM posts WHERE status = 2")
            .fetch_one(pool)
            .await
            .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))?
    };

    let rows: Vec<(
        i64,
        i64,
        Option<i64>,
        String,
        String,
        String,
        i32,
        i64,
        i64,
        String,
        Option<String>,
    )> = {
        let _s = state
            .tracing
            .span_with(&trace, "query_posts", "查询文章列表");
        sqlx::query_as(
            "SELECT id, author_id, category_id, title, summary, content, status, view_count, like_count, created_at, published_at FROM posts WHERE status = 2 ORDER BY published_at DESC LIMIT ? OFFSET ?"
        ).bind(page_size).bind(offset).fetch_all(pool).await
            .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))?
    };

    Ok(PostListResp {
        code: 0,
        total: total.0,
        page,
        page_size,
        data: rows
            .into_iter()
            .map(|r| PostItem {
                id: r.0,
                author_id: r.1,
                category_id: r.2.unwrap_or(0),
                title: r.3,
                summary: r.4,
                status: r.6,
                view_count: r.7,
                like_count: r.8,
                created_at: r.9,
                published_at: r.10.unwrap_or_default(),
            })
            .collect(),
    })
}

// ── 文章详情 ──────────────────────────────────────────────────

#[handler(desc("文章详情"))]
async fn get_post(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Data(req): Data<PostIdReq>,
) -> afast::Result<PostDetailResp> {
    let pool = state.db.sqlite();

    {
        let _s = state
            .tracing
            .span_with(&trace, "update_views", "更新浏览量");
        sqlx::query("UPDATE posts SET view_count = view_count + 1 WHERE id = ? AND status = 2")
            .bind(req.post_id)
            .execute(pool)
            .await
            .ok();
    }

    let p: (
        i64,
        i64,
        Option<i64>,
        String,
        String,
        String,
        String,
        i32,
        i64,
        i64,
        bool,
        String,
        String,
        Option<String>,
    ) = {
        let _s = state.tracing.span_with(&trace, "query_post", "查询文章");
        sqlx::query_as(
            "SELECT id, author_id, category_id, title, slug, summary, content, status, view_count, like_count, is_pinned, created_at, updated_at, published_at FROM posts WHERE id = ? AND status = 2"
        ).bind(req.post_id).fetch_one(pool).await
            .map_err(|_| afaster::Error::custom(40001, "文章不存在"))?
    };

    let tags: Vec<(String,)> = {
        let _s = state.tracing.span_with(&trace, "query_tags", "查询标签");
        sqlx::query_as(
            "SELECT t.name FROM tags t JOIN post_tags pt ON t.id = pt.tag_id WHERE pt.post_id = ?",
        )
        .bind(req.post_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    };

    let comment_count: (i64,) = {
        let _s = state
            .tracing
            .span_with(&trace, "count_comments", "统计评论");
        sqlx::query_as("SELECT COUNT(*) FROM comments WHERE post_id = ? AND status = 0")
            .bind(req.post_id)
            .fetch_one(pool)
            .await
            .unwrap_or((0,))
    };

    Ok(PostDetailResp {
        code: 0,
        id: p.0,
        author_id: p.1,
        category_id: p.2.unwrap_or(0),
        title: p.3,
        slug: p.4,
        summary: p.5,
        content: p.6,
        status: p.7,
        view_count: p.8,
        like_count: p.9,
        is_pinned: p.10,
        created_at: p.11,
        published_at: p.13.unwrap_or_default(),
        tags: tags.into_iter().map(|t| t.0).collect(),
        comment_count: comment_count.0,
    })
}

// ── 分类 / 标签列表 ──────────────────────────────────────────

#[handler(desc("分类列表"))]
async fn list_categories(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
) -> afast::Result<CategoryListResp> {
    let pool = state.db.sqlite();
    let rows: Vec<(i64, String, String, String, i32)> = {
        let _s = state
            .tracing
            .span_with(&trace, "query_categories", "查询分类");
        sqlx::query_as("SELECT id, name, slug, description, sort_order FROM categories ORDER BY sort_order ASC")
            .fetch_all(pool).await
            .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))?
    };
    Ok(CategoryListResp {
        code: 0,
        data: rows
            .into_iter()
            .map(|r| CategoryItem {
                id: r.0,
                name: r.1,
                slug: r.2,
                description: r.3,
                sort_order: r.4,
            })
            .collect(),
    })
}

#[handler(desc("标签列表"))]
async fn list_tags(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
) -> afast::Result<TagListResp> {
    let pool = state.db.sqlite();
    let rows: Vec<(i64, String, String)> = {
        let _s = state.tracing.span_with(&trace, "query_tags", "查询标签");
        sqlx::query_as("SELECT id, name, slug FROM tags ORDER BY id ASC")
            .fetch_all(pool)
            .await
            .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))?
    };
    Ok(TagListResp {
        code: 0,
        data: rows
            .into_iter()
            .map(|r| TagItem {
                id: r.0,
                name: r.1,
                slug: r.2,
            })
            .collect(),
    })
}

// ── 仪表盘（复杂链路测试） ──────────────────────────────────────

#[handler(desc("仪表盘 - 复杂链路"))]
async fn dashboard(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Data(req): Data<DashboardReq>,
) -> afast::Result<DashboardResp> {
    let pool = state.db.sqlite();
    let trace_id = &trace.trace_id;
    let root_span_id = &trace.span_id;
    let t0 = now_ms();
    let mut cursor = t0;

    let s1_id = uuid();
    let user_info: Option<(i64, String, String)> =
        sqlx::query_as("SELECT id, username, nickname FROM users WHERE id = ? AND status = 0")
            .bind(req.user_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))?;
    let d1 = now_ms() - cursor;
    report_span_dur(
        &state,
        trace_id,
        &s1_id,
        Some(root_span_id),
        "auth",
        "鉴权",
        cursor,
        d1,
        &trace.transport,
    )
    .await;
    cursor += d1;

    let s2_id = uuid();
    let post_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE author_id = ?")
        .bind(req.user_id)
        .fetch_one(pool)
        .await
        .unwrap_or((0,));
    let comment_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM comments WHERE author_id = ?")
        .bind(req.user_id)
        .fetch_one(pool)
        .await
        .unwrap_or((0,));
    let d2 = now_ms() - cursor;
    report_span_dur(
        &state,
        &trace_id,
        &s2_id,
        Some(root_span_id),
        "query_user_stats",
        "查询用户统计",
        cursor,
        d2,
        &trace.transport,
    )
    .await;
    cursor += d2;

    let s3_id = uuid();
    let rows: Vec<(i64, i64, Option<i64>, String, String, String, i32, i64, i64, String, Option<String>)> = sqlx::query_as(
        "SELECT id, author_id, category_id, title, summary, '', status, view_count, like_count, created_at, published_at FROM posts WHERE author_id = ? ORDER BY created_at DESC LIMIT 5",
    ).bind(req.user_id).fetch_all(pool).await.map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))?;
    let d3 = now_ms() - cursor;
    report_span_dur(
        &state,
        &trace_id,
        &s3_id,
        Some(root_span_id),
        "query_recent_posts",
        "查询最近文章",
        cursor,
        d3,
        &trace.transport,
    )
    .await;
    cursor += d3;

    let s4_id = uuid();
    let cat_rows: Vec<(i64, String, String, String, i32)> = sqlx::query_as(
        "SELECT id, name, slug, description, sort_order FROM categories ORDER BY sort_order ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))?;
    let d4 = now_ms() - cursor;
    report_span_dur(
        &state,
        &trace_id,
        &s4_id,
        Some(root_span_id),
        "query_categories",
        "查询分类",
        cursor,
        d4,
        &trace.transport,
    )
    .await;
    cursor += d4;

    let s5_id = uuid();
    let tag_rows: Vec<(i64, String, String)> =
        sqlx::query_as("SELECT id, name, slug FROM tags ORDER BY id ASC")
            .fetch_all(pool)
            .await
            .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))?;
    let d5 = now_ms() - cursor;
    report_span_dur(
        &state,
        &trace_id,
        &s5_id,
        Some(root_span_id),
        "query_tags",
        "查询标签",
        cursor,
        d5,
        &trace.transport,
    )
    .await;
    cursor += d5;

    let s6_id = uuid();
    let total_posts: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts")
        .fetch_one(pool)
        .await
        .unwrap_or((0,));
    let total_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .unwrap_or((0,));
    let total_comments: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM comments")
        .fetch_one(pool)
        .await
        .unwrap_or((0,));
    let d6 = now_ms() - cursor;
    report_span_dur(
        &state,
        &trace_id,
        &s6_id,
        Some(root_span_id),
        "query_site_stats",
        "全站统计",
        cursor,
        d6,
        &trace.transport,
    )
    .await;
    cursor += d6;

    let s7_id = uuid();
    let dashboard_user = user_info.map(|u| DashboardUser {
        id: u.0,
        username: u.1,
        nickname: u.2,
        post_count: post_count.0,
        comment_count: comment_count.0,
    });
    let recent_posts = rows
        .into_iter()
        .map(|r| PostItem {
            id: r.0,
            author_id: r.1,
            category_id: r.2.unwrap_or(0),
            title: r.3,
            summary: r.4,
            status: r.6,
            view_count: r.7,
            like_count: r.8,
            created_at: r.9,
            published_at: r.10.unwrap_or_default(),
        })
        .collect();
    let total_categories = cat_rows.len() as i64;
    let total_tags = tag_rows.len() as i64;
    let categories = cat_rows
        .into_iter()
        .map(|r| CategoryItem {
            id: r.0,
            name: r.1,
            slug: r.2,
            description: r.3,
            sort_order: r.4,
        })
        .collect();
    let tags = tag_rows
        .into_iter()
        .map(|r| TagItem {
            id: r.0,
            name: r.1,
            slug: r.2,
        })
        .collect();
    let d7 = now_ms() - cursor;
    report_span_dur(
        &state,
        &trace_id,
        &s7_id,
        Some(root_span_id),
        "assemble_response",
        "组装响应",
        cursor,
        d7,
        &trace.transport,
    )
    .await;

    Ok(DashboardResp {
        code: 0,
        user: dashboard_user,
        recent_posts,
        categories,
        tags,
        stats: DashboardStats {
            total_posts: total_posts.0,
            total_users: total_users.0,
            total_comments: total_comments.0,
            total_categories,
            total_tags,
        },
    })
}

async fn report_span_dur(
    state: &afaster::AppState,
    trace_id: &str,
    span_id: &str,
    parent_id: Option<&str>,
    handler: &str,
    desc: &str,
    start_ms: i64,
    duration_us: i64,
    transport: &str,
) {
    use afaster::trace::store::{SpanData, SpanStatus};
    let span = SpanData {
        trace_id: trace_id.to_string(),
        span_id: span_id.to_string(),
        parent_span_id: parent_id.map(|s| s.to_string()),
        handler_name: handler.to_string(),
        handler_desc: desc.to_string(),
        transport: transport.to_string(),
        is_binary: false,
        method: String::new(),
        long_connection: false,
        is_root: false,
        start_time: start_ms,
        duration_us: duration_us.max(1),
        status: SpanStatus::Ok,
        error_code: None,
        error_message: None,
        service_name: state.tracing.config().service_name.clone(),
    };
    let _ = state.tracing.store().insert_span(span).await;
}

// ── 数据报告（深度嵌套链路测试） ─────────────────────────────────

#[handler(desc("数据报告 - 深度嵌套"))]
async fn generate_report(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
) -> afast::Result<DataReportResp> {
    let pool = state.db.sqlite();

    // ── Level 1: 验证输入 ──
    {
        let _s = state
            .tracing
            .span_with(&trace, "validate_input", "验证输入参数");
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }

    // ── Level 1: 获取数据 ──
    let (total_users, total_posts, total_comments, users, posts) = {
        let s1 = state
            .tracing
            .span_with(&trace, "fetch_all_data", "获取全部数据");
        let ctx1 = s1.child_ctx();

        let total_users: (i64,) = {
            let _s = state.tracing.span_with(&ctx1, "query_users", "查询用户数");
            sqlx::query_as("SELECT COUNT(*) FROM users")
                .fetch_one(pool)
                .await
                .unwrap_or((0,))
        };
        let total_posts: (i64,) = {
            let _s = state.tracing.span_with(&ctx1, "query_posts", "查询文章数");
            sqlx::query_as("SELECT COUNT(*) FROM posts")
                .fetch_one(pool)
                .await
                .unwrap_or((0,))
        };
        let total_comments: (i64,) = {
            let _s = state
                .tracing
                .span_with(&ctx1, "query_comments", "查询评论数");
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            sqlx::query_as("SELECT COUNT(*) FROM comments")
                .fetch_one(pool)
                .await
                .unwrap_or((0,))
        };
        let users: Vec<(i64, String, String)> = {
            let _s = state
                .tracing
                .span_with(&ctx1, "query_user_list", "查询用户列表");
            sqlx::query_as("SELECT id, username, nickname FROM users LIMIT 10")
                .fetch_all(pool)
                .await
                .unwrap_or_default()
        };
        let posts: Vec<(i64, String, i64, i64)> = {
            let _s = state
                .tracing
                .span_with(&ctx1, "query_post_list", "查询文章列表");
            sqlx::query_as(
                "SELECT id, title, view_count, like_count FROM posts WHERE status = 2 LIMIT 10",
            )
            .fetch_all(pool)
            .await
            .unwrap_or_default()
        };

        (total_users, total_posts, total_comments, users, posts)
    }; // s1 drop → 自动上报

    // ── Level 1: 处理数据 ──
    let (metrics, filtered_posts, summary) = {
        let s1 = state.tracing.span_with(&trace, "process_data", "处理数据");
        let ctx1 = s1.child_ctx();

        // Level 2: 计算指标
        let metrics = {
            let s2 = state
                .tracing
                .span_with(&ctx1, "compute_metrics", "计算指标");
            let ctx2 = s2.child_ctx();

            let avg_views = {
                let _s = state
                    .tracing
                    .span_with(&ctx2, "calc_avg_views", "计算平均浏览量");
                tokio::time::sleep(std::time::Duration::from_millis(3)).await;
                if posts.is_empty() {
                    0.0
                } else {
                    posts.iter().map(|p| p.2 as f64).sum::<f64>() / posts.len() as f64
                }
            };
            let avg_likes = {
                let _s = state
                    .tracing
                    .span_with(&ctx2, "calc_avg_likes", "计算平均点赞数");
                if posts.is_empty() {
                    0.0
                } else {
                    posts.iter().map(|p| p.3 as f64).sum::<f64>() / posts.len() as f64
                }
            };
            let engagement_rate = {
                let _s = state
                    .tracing
                    .span_with(&ctx2, "calc_engagement_rate", "计算互动率");
                tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                if avg_views > 0.0 {
                    (avg_likes / avg_views * 100.0 * 100.0).round() / 100.0
                } else {
                    0.0
                }
            };

            MetricsData {
                avg_views,
                avg_likes,
                engagement_rate,
            }
        }; // s2 drop

        // Level 2: 过滤
        let filtered_posts = {
            let _s = state
                .tracing
                .span_with(&ctx1, "apply_filters", "应用过滤条件");
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            posts
                .into_iter()
                .filter(|p| p.2 > 0)
                .map(|p| PostItem {
                    id: p.0,
                    author_id: 0,
                    category_id: 0,
                    title: p.1,
                    summary: String::new(),
                    status: 2,
                    view_count: p.2,
                    like_count: p.3,
                    created_at: String::new(),
                    published_at: String::new(),
                })
                .collect::<Vec<_>>()
        };

        // Level 2: 构建摘要
        let summary = {
            let _s = state.tracing.span_with(&ctx1, "build_summary", "构建摘要");
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            format!(
                "共 {} 用户, {} 文章, {} 评论, 平均浏览 {:.1}, 互动率 {}%",
                total_users.0,
                total_posts.0,
                total_comments.0,
                metrics.avg_views,
                metrics.engagement_rate
            )
        };

        (metrics, filtered_posts, summary)
    };

    // ── Level 1: 格式化输出 ──
    {
        let _s = state
            .tracing
            .span_with(&trace, "format_output", "格式化输出");
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }

    Ok(DataReportResp {
        code: 0,
        total_users: total_users.0,
        total_posts: total_posts.0,
        total_comments: total_comments.0,
        users: users
            .into_iter()
            .map(|u| DashboardUser {
                id: u.0,
                username: u.1,
                nickname: u.2,
                post_count: 0,
                comment_count: 0,
            })
            .collect(),
        top_posts: filtered_posts,
        summary,
        avg_views: metrics.avg_views,
        avg_likes: metrics.avg_likes,
        engagement_rate: metrics.engagement_rate,
    })
}

struct MetricsData {
    avg_views: f64,
    avg_likes: f64,
    engagement_rate: f64,
}

fn uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{:016x}", t.as_nanos() & 0xFFFFFFFFFFFFFFFF)
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

// ── 支付模拟（错误链路测试） ──────────────────────────────────────

#[handler(desc("支付模拟 - 含错误"))]
async fn simulate_payment(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Data(req): Data<PostIdReq>,
) -> afast::Result<ApiResp> {
    let pool = state.db.sqlite();

    // ✅ 查询订单
    let post = {
        let _s = state.tracing.span_with(&trace, "query_order", "查询订单");
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        sqlx::query_as::<_, (i64, String)>("SELECT id, title FROM posts WHERE id = ?")
            .bind(req.post_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))?
    };

    let _post = post.ok_or_else(|| afaster::Error::custom(40004, "订单不存在"))?;

    // ✅ 验证金额
    {
        let _s = state.tracing.span_with(&trace, "verify_amount", "验证金额");
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }

    // ❌ 调用支付网关（模拟失败）
    {
        let _s = state
            .tracing
            .span_with(&trace, "call_payment_gateway", "调用支付网关");
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        // 模拟支付网关超时
        return Err(afaster::Error::custom(50301, "支付网关超时").into());
    }
}

#[handler(desc("批量查询 - 部分失败"))]
async fn batch_query(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
) -> afast::Result<ApiResp> {
    let pool = state.db.sqlite();

    // ✅ 查询用户
    {
        let _s = state.tracing.span_with(&trace, "query_users", "查询用户");
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        let _: Vec<(i64,)> = sqlx::query_as("SELECT id FROM users LIMIT 5")
            .fetch_all(pool)
            .await
            .unwrap_or_default();
    }

    // ✅ 查询文章
    {
        let _s = state.tracing.span_with(&trace, "query_posts", "查询文章");
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        let _: Vec<(i64,)> = sqlx::query_as("SELECT id FROM posts LIMIT 5")
            .fetch_all(pool)
            .await
            .unwrap_or_default();
    }

    // ❌ 查询不存在的表
    {
        let mut _s = state.tracing.span_with(&trace, "query_orders", "查询订单");
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        let result = sqlx::query_as::<_, (i64,)>("SELECT id FROM orders LIMIT 5")
            .fetch_all(pool)
            .await;
        if result.is_err() {
            _s.set_error(50002, "表 orders 不存在");
        }
    }

    // ✅ 查询评论
    {
        let _s = state
            .tracing
            .span_with(&trace, "query_comments", "查询评论");
        let _: Vec<(i64,)> = sqlx::query_as("SELECT id FROM comments LIMIT 5")
            .fetch_all(pool)
            .await
            .unwrap_or_default();
    }

    Ok(ApiResp::ok("批量查询完成（部分失败）"))
}

/// 构建用户服务
pub fn build_service() -> afast::Service {
    afast::service!("user", "用户接口" => {
        h(register),
        h(login),
        h(list_posts),
        h(get_post),
        h(list_categories),
        h(list_tags),
        h(dashboard),
        h(generate_report),
        h(simulate_payment),
        h(batch_query),
    })
}
