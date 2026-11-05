//! 用户公开接口：注册、登录、浏览文章、分类、标签

use crate::model::entities::{Category, Comment, Post, PostTag, Tag, User, UserFindBy as _};
use crate::model::*;
use afast::{Ctx, Data, State, handler};
use afaster::trace::TraceContext;
use afastorm::Field;

// ── 注册 ──────────────────────────────────────────────────────

#[handler(desc("用户注册"))]
async fn register(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Data(req): Data<RegisterReq>,
) -> afast::Result<ApiResp> {
    let user = {
        let _s = state
            .tracing
            .span_with(&trace, "check_username", "检查用户名");
        state
            .orm
            .repo::<User>()
            .map_err(|error| afaster::Error::custom(50002, error.to_string()))?
            .find_by_username(req.username.clone())
            .await
            .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))?
    };
    if user.is_some() {
        return Ok(ApiResp::err(40001, "用户名已存在"));
    }

    let hash = {
        let _s = state.tracing.span_with(&trace, "hash_password", "哈希密码");
        afaster::argon2::Argon2Hasher::new()
            .hash(&req.password)
            .map_err(|e| afaster::Error::custom(52402, format!("Hash: {}", e)))?
    };

    let user_id = {
        let _s = state.tracing.span_with(&trace, "insert_user", "写入用户");
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let mut user = User {
            id: None,
            username: req.username.clone(),
            nickname: req.nickname.clone(),
            email: req.email.clone(),
            password_hash: hash,
            avatar: String::new(),
            bio: String::new(),
            status: 0,
            created_at: now.clone(),
            updated_at: now,
        };
        state
            .orm
            .repo::<User>()
            .map_err(|error| afaster::Error::custom(50002, error.to_string()))?
            .insert(&mut user)
            .await
            .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))?;
        user.id
            .ok_or_else(|| afaster::Error::custom(50002, "创建用户未返回 ID"))?
    };

    {
        let _s = state.tracing.span_with(&trace, "assign_role", "分配角色");
        if let Some(ref rbac) = state.rbac {
            rbac.assign_role(user_id, "user").await?;
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
    let user = {
        let _s = state.tracing.span_with(&trace, "db_query_user", "查询用户");
        state
            .orm
            .repo::<User>()
            .map_err(|error| afaster::Error::custom(50002, error.to_string()))?
            .find_by_username(req.username.clone())
            .await
            .map_err(|error| afaster::Error::custom(50002, error.to_string()))?
            .ok_or_else(|| afaster::Error::custom(40001, "用户名或密码错误"))?
    };
    let user_id = user
        .id
        .ok_or_else(|| afaster::Error::custom(50002, "用户记录没有主键"))?;
    if user.status != 0 {
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
            .verify(&req.password, &user.password_hash)
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
            let r = rbac.get_user_roles(user_id).await.unwrap_or_default();
            let p = rbac.get_user_permissions(user_id).await.unwrap_or_default();
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
                user_id,
                afaster::token::Data1 {
                    id: user_id,
                    username: user.username.clone(),
                    nickname: user.nickname.clone(),
                    avatar: user.avatar.clone(),
                    is_super: roles.contains(&"super_admin".to_string()),
                },
            )
            .map_err(|e| afaster::Error::custom(50101, format!("JWT: {}", e)))?
    };

    Ok(LoginResp {
        code: 0,
        message: "登录成功".into(),
        token,
        user_id,
        username: user.username,
        nickname: user.nickname,
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
    let page = req.page.max(1);
    let page_size = req.page_size.clamp(1, 100);
    let offset = (page - 1) * page_size;

    let repository = state.orm.repo::<Post>().map_err(db_error)?;
    let total = {
        let _s = state.tracing.span_with(&trace, "count_posts", "统计文章");
        repository
            .count_where(Field::new("status").eq(2))
            .await
            .map_err(db_error)?
    };

    let rows = {
        let _s = state
            .tracing
            .span_with(&trace, "query_posts", "查询文章列表");
        let statement = repository
            .query()
            .filter(Field::new("status").eq(2))
            .order_by(Post::published_at.desc())
            .limit(page_size as u64)
            .offset(offset as u64)
            .build()
            .map_err(db_error)?;
        repository.fetch_all(&statement).await.map_err(db_error)?
    };

    Ok(PostListResp {
        code: 0,
        total,
        page,
        page_size,
        data: rows
            .into_iter()
            .map(|post| PostItem {
                id: post.id.unwrap_or_default(),
                author_id: post.author_id,
                category_id: post.category_id.unwrap_or_default(),
                title: post.title,
                summary: post.summary,
                status: post.status,
                view_count: post.view_count,
                like_count: post.like_count,
                created_at: post.created_at,
                published_at: post.published_at.unwrap_or_default(),
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
    let post = {
        let _s = state
            .tracing
            .span_with(&trace, "update_views", "更新浏览量");
        state
            .orm
            .query("UPDATE posts SET view_count = view_count + 1 WHERE id = ? AND status = 2")
            .bind(req.post_id)
            .execute()
            .await
            .map_err(db_error)?;
        state
            .orm
            .repo::<Post>()
            .map_err(db_error)?
            .find_by_id(req.post_id)
            .await
            .map_err(db_error)?
            .filter(|post| post.status == 2)
            .ok_or_else(|| afaster::Error::custom(40001, "文章不存在"))?
    };

    let tags = {
        let _s = state.tracing.span_with(&trace, "query_tags", "查询标签");
        let statement = state
            .orm
            .repo::<PostTag>()
            .map_err(db_error)?
            .query()
            .filter(Field::new("post_id").eq(req.post_id))
            .build()
            .map_err(db_error)?;
        let links = state
            .orm
            .fetch_all::<PostTag>(&statement)
            .await
            .map_err(db_error)?;
        let repository = state.orm.repo::<Tag>().map_err(db_error)?;
        let mut names = Vec::new();
        for link in links {
            if let Some(tag) = repository.find_by_id(link.tag_id).await.map_err(db_error)? {
                names.push(tag.name);
            }
        }
        names
    };

    let comment_count = {
        let _s = state
            .tracing
            .span_with(&trace, "count_comments", "统计评论");
        state
            .orm
            .repo::<Comment>()
            .map_err(db_error)?
            .count_where(
                Field::new("post_id")
                    .eq(req.post_id)
                    .and(Field::new("status").eq(0)),
            )
            .await
            .map_err(db_error)?
    };

    Ok(PostDetailResp {
        code: 0,
        id: post.id.unwrap_or_default(),
        author_id: post.author_id,
        category_id: post.category_id.unwrap_or_default(),
        title: post.title,
        slug: post.slug,
        summary: post.summary,
        content: post.content,
        status: post.status,
        view_count: post.view_count,
        like_count: post.like_count,
        is_pinned: post.is_pinned,
        created_at: post.created_at,
        published_at: post.published_at.unwrap_or_default(),
        tags,
        comment_count,
    })
}

// ── 分类 / 标签列表 ──────────────────────────────────────────

#[handler(desc("分类列表"))]
async fn list_categories(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
) -> afast::Result<CategoryListResp> {
    let rows = {
        let _s = state
            .tracing
            .span_with(&trace, "query_categories", "查询分类");
        let statement = state
            .orm
            .repo::<Category>()
            .map_err(db_error)?
            .query()
            .order_by(Category::sort_order.asc())
            .build()
            .map_err(db_error)?;
        state
            .orm
            .fetch_all::<Category>(&statement)
            .await
            .map_err(db_error)?
    };
    Ok(CategoryListResp {
        code: 0,
        data: rows
            .into_iter()
            .map(|row| CategoryItem {
                id: row.id.unwrap_or_default(),
                name: row.name,
                slug: row.slug,
                description: row.description,
                sort_order: row.sort_order,
            })
            .collect(),
    })
}

#[handler(desc("标签列表"))]
async fn list_tags(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
) -> afast::Result<TagListResp> {
    let rows = {
        let _s = state.tracing.span_with(&trace, "query_tags", "查询标签");
        let statement = state
            .orm
            .repo::<Tag>()
            .map_err(db_error)?
            .query()
            .order_by(Tag::id.asc())
            .build()
            .map_err(db_error)?;
        state
            .orm
            .fetch_all::<Tag>(&statement)
            .await
            .map_err(db_error)?
    };
    Ok(TagListResp {
        code: 0,
        data: rows
            .into_iter()
            .map(|row| TagItem {
                id: row.id.unwrap_or_default(),
                name: row.name,
                slug: row.slug,
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
    let trace_id = &trace.trace_id;
    let root_span_id = &trace.span_id;
    let t0 = now_ms();
    let mut cursor = t0;

    let s1_id = uuid();
    let user_info = state
        .orm
        .repo::<User>()
        .map_err(db_error)?
        .find_by_id(req.user_id)
        .await
        .map_err(db_error)?
        .filter(|user| user.status == 0);
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
    let post_count = state
        .orm
        .repo::<Post>()
        .map_err(db_error)?
        .count_where(Field::new("author_id").eq(req.user_id))
        .await
        .map_err(db_error)?;
    let comment_count = state
        .orm
        .repo::<Comment>()
        .map_err(db_error)?
        .count_where(Field::new("author_id").eq(req.user_id))
        .await
        .map_err(db_error)?;
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
    let post_repository = state.orm.repo::<Post>().map_err(db_error)?;
    let recent_posts_statement = post_repository
        .query()
        .filter(Field::new("author_id").eq(req.user_id))
        .order_by(Post::created_at.desc())
        .limit(5)
        .build()
        .map_err(db_error)?;
    let rows = post_repository
        .fetch_all(&recent_posts_statement)
        .await
        .map_err(db_error)?;
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
    let categories_statement = state
        .orm
        .repo::<Category>()
        .map_err(db_error)?
        .query()
        .order_by(Category::sort_order.asc())
        .build()
        .map_err(db_error)?;
    let cat_rows = state
        .orm
        .fetch_all::<Category>(&categories_statement)
        .await
        .map_err(db_error)?;
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
    let tags_statement = state
        .orm
        .repo::<Tag>()
        .map_err(db_error)?
        .query()
        .order_by(Tag::id.asc())
        .build()
        .map_err(db_error)?;
    let tag_rows = state
        .orm
        .fetch_all::<Tag>(&tags_statement)
        .await
        .map_err(db_error)?;
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
    let total_posts = state
        .orm
        .repo::<Post>()
        .map_err(db_error)?
        .count()
        .await
        .map_err(db_error)?;
    let total_users = state
        .orm
        .repo::<User>()
        .map_err(db_error)?
        .count()
        .await
        .map_err(db_error)?;
    let total_comments = state
        .orm
        .repo::<Comment>()
        .map_err(db_error)?
        .count()
        .await
        .map_err(db_error)?;
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
    let dashboard_user = user_info.map(|user| DashboardUser {
        id: user.id.unwrap_or_default(),
        username: user.username,
        nickname: user.nickname,
        post_count,
        comment_count,
    });
    let recent_posts = rows
        .into_iter()
        .map(|post| PostItem {
            id: post.id.unwrap_or_default(),
            author_id: post.author_id,
            category_id: post.category_id.unwrap_or_default(),
            title: post.title,
            summary: post.summary,
            status: post.status,
            view_count: post.view_count,
            like_count: post.like_count,
            created_at: post.created_at,
            published_at: post.published_at.unwrap_or_default(),
        })
        .collect();
    let total_categories = cat_rows.len() as i64;
    let total_tags = tag_rows.len() as i64;
    let categories = cat_rows
        .into_iter()
        .map(|row| CategoryItem {
            id: row.id.unwrap_or_default(),
            name: row.name,
            slug: row.slug,
            description: row.description,
            sort_order: row.sort_order,
        })
        .collect();
    let tags = tag_rows
        .into_iter()
        .map(|row| TagItem {
            id: row.id.unwrap_or_default(),
            name: row.name,
            slug: row.slug,
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
            total_posts,
            total_users,
            total_comments,
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

        let total_users = {
            let _s = state.tracing.span_with(&ctx1, "query_users", "查询用户数");
            state
                .orm
                .repo::<User>()
                .map_err(db_error)?
                .count()
                .await
                .map_err(db_error)?
        };
        let total_posts = {
            let _s = state.tracing.span_with(&ctx1, "query_posts", "查询文章数");
            state
                .orm
                .repo::<Post>()
                .map_err(db_error)?
                .count()
                .await
                .map_err(db_error)?
        };
        let total_comments = {
            let _s = state
                .tracing
                .span_with(&ctx1, "query_comments", "查询评论数");
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            state
                .orm
                .repo::<Comment>()
                .map_err(db_error)?
                .count()
                .await
                .map_err(db_error)?
        };
        let users = {
            let _s = state
                .tracing
                .span_with(&ctx1, "query_user_list", "查询用户列表");
            let statement = state
                .orm
                .repo::<User>()
                .map_err(db_error)?
                .query()
                .limit(10)
                .build()
                .map_err(db_error)?;
            state
                .orm
                .fetch_all::<User>(&statement)
                .await
                .map_err(db_error)?
        };
        let posts = {
            let _s = state
                .tracing
                .span_with(&ctx1, "query_post_list", "查询文章列表");
            let repository = state.orm.repo::<Post>().map_err(db_error)?;
            let statement = repository
                .query()
                .filter(Field::new("status").eq(2))
                .limit(10)
                .build()
                .map_err(db_error)?;
            repository.fetch_all(&statement).await.map_err(db_error)?
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
                    posts.iter().map(|post| post.view_count as f64).sum::<f64>()
                        / posts.len() as f64
                }
            };
            let avg_likes = {
                let _s = state
                    .tracing
                    .span_with(&ctx2, "calc_avg_likes", "计算平均点赞数");
                if posts.is_empty() {
                    0.0
                } else {
                    posts.iter().map(|post| post.like_count as f64).sum::<f64>()
                        / posts.len() as f64
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
                .filter(|post| post.view_count > 0)
                .map(|post| PostItem {
                    id: post.id.unwrap_or_default(),
                    author_id: post.author_id,
                    category_id: post.category_id.unwrap_or_default(),
                    title: post.title,
                    summary: post.summary,
                    status: post.status,
                    view_count: post.view_count,
                    like_count: post.like_count,
                    created_at: post.created_at,
                    published_at: post.published_at.unwrap_or_default(),
                })
                .collect::<Vec<_>>()
        };

        // Level 2: 构建摘要
        let summary = {
            let _s = state.tracing.span_with(&ctx1, "build_summary", "构建摘要");
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            format!(
                "共 {} 用户, {} 文章, {} 评论, 平均浏览 {:.1}, 互动率 {}%",
                total_users,
                total_posts,
                total_comments,
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
        total_users,
        total_posts,
        total_comments,
        users: users
            .into_iter()
            .map(|user| DashboardUser {
                id: user.id.unwrap_or_default(),
                username: user.username,
                nickname: user.nickname,
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

fn db_error(error: impl std::fmt::Display) -> afast::Error {
    afaster::Error::custom(50002, format!("DB: {error}")).into()
}

// ── 支付模拟（错误链路测试） ──────────────────────────────────────

#[handler(desc("支付模拟 - 含错误"))]
async fn simulate_payment(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Data(req): Data<PostIdReq>,
) -> afast::Result<ApiResp> {
    // ✅ 查询订单
    let post = {
        let _s = state.tracing.span_with(&trace, "query_order", "查询订单");
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        state
            .orm
            .repo::<Post>()
            .map_err(db_error)?
            .find_by_id(req.post_id)
            .await
            .map_err(db_error)?
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
    // ✅ 查询用户
    {
        let _s = state.tracing.span_with(&trace, "query_users", "查询用户");
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        let statement = state
            .orm
            .repo::<User>()
            .map_err(db_error)?
            .query()
            .limit(5)
            .build()
            .map_err(db_error)?;
        let _: Vec<User> = state.orm.fetch_all(&statement).await.map_err(db_error)?;
    }

    // ✅ 查询文章
    {
        let _s = state.tracing.span_with(&trace, "query_posts", "查询文章");
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        let repository = state.orm.repo::<Post>().map_err(db_error)?;
        let statement = repository.query().limit(5).build().map_err(db_error)?;
        let _: Vec<Post> = repository.fetch_all(&statement).await.map_err(db_error)?;
    }

    // ❌ 查询不存在的表
    {
        let mut _s = state.tracing.span_with(&trace, "query_orders", "查询订单");
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        let result = state
            .orm
            .query_as::<(i64,)>("SELECT id FROM orders LIMIT 5")
            .fetch_all()
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
        let statement = state
            .orm
            .repo::<Comment>()
            .map_err(db_error)?
            .query()
            .limit(5)
            .build()
            .map_err(db_error)?;
        let _: Vec<Comment> = state.orm.fetch_all(&statement).await.map_err(db_error)?;
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
