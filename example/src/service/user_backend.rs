//! 用户后台：管理自己的文章、评论、点赞

use crate::model::*;
use afast::{Ctx, Custom, Data, State, handler};
use afaster::trace::TraceContext;

// ── 个人信息 ──────────────────────────────────────────────────

#[handler(desc("个人信息"))]
async fn me(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Custom(auth): Custom<AuthToken>,
) -> afast::Result<LoginResp> {
    let uid = state
        .token
        .get_id(&auth.token)
        .map_err(|_| afaster::Error::custom(40101, "无效的令牌"))?;
    let pool = &state.db.pool;

    let u: (i64, String, String, String, String, String, i32) = {
        let _s = state.tracing.span_with(&trace, "query_user", "查询用户");
        sqlx::query_as(
            "SELECT id, username, nickname, email, avatar, bio, status FROM users WHERE id = ?",
        )
        .bind(uid)
        .fetch_one(pool)
        .await
        .map_err(|_| afaster::Error::custom(40001, "用户不存在"))?
    };

    let (roles, permissions) = {
        let _s = state
            .tracing
            .span_with(&trace, "query_permissions", "查询权限");
        if let Some(ref rbac) = state.rbac {
            let r = rbac.get_user_roles(uid).await.unwrap_or_default();
            let p = rbac.get_user_permissions(uid).await.unwrap_or_default();
            (r.iter().map(|x| x.name.clone()).collect(), p)
        } else {
            (vec![], vec![])
        }
    };

    Ok(LoginResp {
        code: 0,
        message: "ok".into(),
        token: String::new(),
        user_id: u.0,
        username: u.1,
        nickname: u.2,
        roles,
        permissions,
    })
}

// ── 创建文章 ──────────────────────────────────────────────────

#[handler(desc("创建文章"))]
async fn create_post(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Custom(auth): Custom<AuthToken>,
    Data(req): Data<CreatePostReq>,
) -> afast::Result<ApiResp> {
    let uid = state
        .token
        .get_id(&auth.token)
        .map_err(|_| afaster::Error::custom(40101, "无效的令牌"))?;

    {
        let _s = state
            .tracing
            .span_with(&trace, "check_permission", "检查权限");
        if let Some(ref rbac) = state.rbac {
            if !rbac
                .check_permission(uid, "post:create")
                .await
                .unwrap_or(false)
            {
                return Ok(ApiResp::err(40301, "权限不足"));
            }
        }
    }

    let pool = &state.db.pool;
    let slug = req
        .title
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect::<String>()
        .replace(' ', "-")
        .to_lowercase();
    let summary = if req.summary.is_empty() {
        req.content.chars().take(200).collect()
    } else {
        req.summary
    };

    let can_publish = if let Some(ref rbac) = state.rbac {
        rbac.check_permission(uid, "post:publish")
            .await
            .unwrap_or(false)
    } else {
        true
    };
    let status: i32 = if can_publish { 2 } else { 1 };
    let published_at = if status == 2 {
        Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
    } else {
        None
    };

    let post_id: (i64,) = {
        let _s = state.tracing.span_with(&trace, "insert_post", "写入文章");
        sqlx::query("INSERT INTO posts (author_id, category_id, title, slug, summary, content, status, published_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(uid).bind(if req.category_id > 0 { Some(req.category_id) } else { None })
            .bind(&req.title).bind(&slug).bind(&summary).bind(&req.content).bind(status).bind(&published_at)
            .execute(pool).await.map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))?;
        sqlx::query_as("SELECT last_insert_rowid()")
            .fetch_one(pool)
            .await
            .unwrap_or((0,))
    };

    {
        let _s = state.tracing.span_with(&trace, "insert_tags", "写入标签");
        for tag_name in &req.tags {
            let tag_id: Option<(i64,)> = sqlx::query_as("SELECT id FROM tags WHERE name = ?")
                .bind(tag_name)
                .fetch_optional(pool)
                .await
                .unwrap_or(None);
            let tid = if let Some(t) = tag_id {
                t.0
            } else {
                let s = tag_name
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == ' ')
                    .collect::<String>()
                    .replace(' ', "-")
                    .to_lowercase();
                sqlx::query("INSERT INTO tags (name, slug) VALUES (?, ?)")
                    .bind(tag_name)
                    .bind(&s)
                    .execute(pool)
                    .await
                    .ok();
                let n: (i64,) = sqlx::query_as("SELECT last_insert_rowid()")
                    .fetch_one(pool)
                    .await
                    .unwrap_or((0,));
                n.0
            };
            sqlx::query("INSERT OR IGNORE INTO post_tags (post_id, tag_id) VALUES (?, ?)")
                .bind(post_id.0)
                .bind(tid)
                .execute(pool)
                .await
                .ok();
        }
    }

    Ok(ApiResp::ok(if status == 2 {
        "发布成功"
    } else {
        "提交成功，等待审核"
    }))
}

// ── 我的文章 ──────────────────────────────────────────────────

#[handler(desc("我的文章"))]
async fn my_posts(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Custom(auth): Custom<AuthToken>,
    Data(req): Data<PageReq>,
) -> afast::Result<PostListResp> {
    let uid = state
        .token
        .get_id(&auth.token)
        .map_err(|_| afaster::Error::custom(40101, "无效的令牌"))?;
    let pool = &state.db.pool;
    let page = req.page.max(1);
    let ps = 20i32;
    let offset = (page - 1) * ps;

    let total: (i64,) = {
        let _s = state.tracing.span_with(&trace, "count_posts", "统计文章");
        sqlx::query_as("SELECT COUNT(*) FROM posts WHERE author_id = ? AND status != 3")
            .bind(uid)
            .fetch_one(pool)
            .await
            .unwrap_or((0,))
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
        let _s = state.tracing.span_with(&trace, "query_posts", "查询文章");
        sqlx::query_as(
            "SELECT id, author_id, category_id, title, summary, content, status, view_count, like_count, created_at, published_at FROM posts WHERE author_id = ? AND status != 3 ORDER BY id DESC LIMIT ? OFFSET ?"
        ).bind(uid).bind(ps).bind(offset).fetch_all(pool).await.unwrap_or_default()
    };

    Ok(PostListResp {
        code: 0,
        total: total.0,
        page,
        page_size: ps,
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

// ── 删除文章 ──────────────────────────────────────────────────

#[handler(desc("删除文章"))]
async fn delete_post(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Custom(auth): Custom<AuthToken>,
    Data(post_id): Data<PostIdReq>,
) -> afast::Result<ApiResp> {
    let uid = state
        .token
        .get_id(&auth.token)
        .map_err(|_| afaster::Error::custom(40101, "无效的令牌"))?;
    let pool = &state.db.pool;

    {
        let _s = state.tracing.span_with(&trace, "check_owner", "检查权限");
        let author: (i64,) = sqlx::query_as("SELECT author_id FROM posts WHERE id = ?")
            .bind(post_id.post_id)
            .fetch_one(pool)
            .await
            .map_err(|_| afaster::Error::custom(40001, "文章不存在"))?;
        if author.0 != uid {
            if let Some(ref rbac) = state.rbac {
                if !rbac
                    .check_permission(uid, "post:delete")
                    .await
                    .unwrap_or(false)
                {
                    return Ok(ApiResp::err(40301, "只能删除自己的文章"));
                }
            }
        }
    }

    {
        let _s = state.tracing.span_with(&trace, "soft_delete", "软删除文章");
        sqlx::query("UPDATE posts SET status = 3, updated_at = datetime('now') WHERE id = ?")
            .bind(post_id.post_id)
            .execute(pool)
            .await
            .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))?;
    }

    Ok(ApiResp::ok("删除成功"))
}

// ── 发表评论 ──────────────────────────────────────────────────

#[handler(desc("发表评论"))]
async fn create_comment(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Custom(auth): Custom<AuthToken>,
    Data(req): Data<CreateCommentReq>,
) -> afast::Result<ApiResp> {
    let uid = state
        .token
        .get_id(&auth.token)
        .map_err(|_| afaster::Error::custom(40101, "无效的令牌"))?;

    {
        let _s = state
            .tracing
            .span_with(&trace, "check_permission", "检查权限");
        if let Some(ref rbac) = state.rbac {
            if !rbac
                .check_permission(uid, "comment:create")
                .await
                .unwrap_or(false)
            {
                return Ok(ApiResp::err(40301, "权限不足"));
            }
        }
    }

    let pool = &state.db.pool;

    {
        let _s = state.tracing.span_with(&trace, "check_post", "检查文章");
        let ps: (i32,) = sqlx::query_as("SELECT status FROM posts WHERE id = ?")
            .bind(req.post_id)
            .fetch_one(pool)
            .await
            .map_err(|_| afaster::Error::custom(40001, "文章不存在"))?;
        if ps.0 != 2 {
            return Ok(ApiResp::err(40002, "文章未发布"));
        }
    }

    {
        let _s = state
            .tracing
            .span_with(&trace, "insert_comment", "写入评论");
        let parent = if req.parent_id > 0 {
            Some(req.parent_id)
        } else {
            None
        };
        sqlx::query(
            "INSERT INTO comments (post_id, author_id, parent_id, content) VALUES (?, ?, ?, ?)",
        )
        .bind(req.post_id)
        .bind(uid)
        .bind(parent)
        .bind(&req.content)
        .execute(pool)
        .await
        .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))?;
    }

    Ok(ApiResp::ok("评论成功"))
}

// ── 点赞 ──────────────────────────────────────────────────────

#[handler(desc("点赞文章"))]
async fn like_post(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Custom(auth): Custom<AuthToken>,
    Data(post_id): Data<PostIdReq>,
) -> afast::Result<ApiResp> {
    let uid = state
        .token
        .get_id(&auth.token)
        .map_err(|_| afaster::Error::custom(40101, "无效的令牌"))?;
    let pool = &state.db.pool;

    let liked: (i64,) = {
        let _s = state.tracing.span_with(&trace, "check_liked", "检查点赞");
        sqlx::query_as("SELECT COUNT(*) FROM post_likes WHERE post_id = ? AND user_id = ?")
            .bind(post_id.post_id)
            .bind(uid)
            .fetch_one(pool)
            .await
            .unwrap_or((0,))
    };

    if liked.0 > 0 {
        let _s = state.tracing.span_with(&trace, "unlike", "取消点赞");
        sqlx::query("DELETE FROM post_likes WHERE post_id = ? AND user_id = ?")
            .bind(post_id.post_id)
            .bind(uid)
            .execute(pool)
            .await
            .ok();
        sqlx::query("UPDATE posts SET like_count = like_count - 1 WHERE id = ?")
            .bind(post_id.post_id)
            .execute(pool)
            .await
            .ok();
        Ok(ApiResp::ok("取消点赞"))
    } else {
        let _s = state.tracing.span_with(&trace, "like", "点赞");
        sqlx::query("INSERT INTO post_likes (post_id, user_id) VALUES (?, ?)")
            .bind(post_id.post_id)
            .bind(uid)
            .execute(pool)
            .await
            .ok();
        sqlx::query("UPDATE posts SET like_count = like_count + 1 WHERE id = ?")
            .bind(post_id.post_id)
            .execute(pool)
            .await
            .ok();
        Ok(ApiResp::ok("点赞成功"))
    }
}

pub fn build_service() -> afast::Service {
    afast::service!("user_backend", "用户后台" => {
        h(me),
        h(create_post),
        h(my_posts),
        h(delete_post),
        h(create_comment),
        h(like_post),
    })
}
