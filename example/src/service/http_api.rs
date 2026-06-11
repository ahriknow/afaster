//! 普通 HTTP REST 接口（ordinary-http）
//!
//! 使用 `#[afast::get]` / `#[afast::post]` 宏，演示 HTTP 接口的链路追踪。

use crate::model::*;
use afast::{Ctx, Json, State};
use afaster::trace::TraceContext;

// ── GET /api/health ────────────────────────────────────────

#[afast::get(desc("健康检查"), no_trace)]
pub async fn health_check() -> Json<ApiResp> {
    Json(ApiResp::ok("ok"))
}

// ── GET /api/categories ────────────────────────────────────

#[afast::get(desc("获取分类列表"))]
pub async fn get_categories(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
) -> Json<CategoryListResp> {
    let pool = state.db.pool();

    let rows: Vec<(i64, String, String, String, i32)> = {
        let _s = state.tracing.span(&trace);
        sqlx::query_as("SELECT id, name, slug, description, sort_order FROM categories ORDER BY sort_order ASC")
            .fetch_all(pool).await.unwrap_or_default()
    };

    Json(CategoryListResp {
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

// ── GET /api/tags ──────────────────────────────────────────

#[afast::get(desc("获取标签列表"))]
pub async fn get_tags(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
) -> Json<TagListResp> {
    let pool = state.db.pool();

    let rows: Vec<(i64, String, String)> = {
        let _s = state.tracing.span(&trace);
        sqlx::query_as("SELECT id, name, slug FROM tags ORDER BY id ASC")
            .fetch_all(pool)
            .await
            .unwrap_or_default()
    };

    Json(TagListResp {
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

// ── GET /api/posts ─────────────────────────────────────────

#[afast::get(desc("获取文章列表"))]
pub async fn get_posts(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
) -> Json<PostListResp> {
    let pool = state.db.pool();

    let total: (i64,) = {
        let _s = state.tracing.span_name(&trace, "count_posts");
        sqlx::query_as("SELECT COUNT(*) FROM posts WHERE status = 2")
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
        let _s = state.tracing.span_name(&trace, "query_posts");
        sqlx::query_as("SELECT id, author_id, category_id, title, summary, content, status, view_count, like_count, created_at, published_at FROM posts WHERE status = 2 ORDER BY published_at DESC LIMIT 50")
            .fetch_all(pool).await.unwrap_or_default()
    };

    Json(PostListResp {
        code: 0,
        total: total.0,
        page: 1,
        page_size: 50,
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

// ── GET /api/posts/:id ─────────────────────────────────────

#[afast::get(desc("获取文章详情"))]
pub async fn get_post_by_id(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    afast::Query(params): afast::Query<PostIdReq>,
) -> Json<PostDetailResp> {
    let pool = state.db.pool();
    let id = params.post_id;

    let post = {
        let _s = state.tracing.span_name(&trace, "query_post");
        sqlx::query_as::<_, (i64, i64, String, String, String, i32, i64, i64)>(
            "SELECT id, author_id, title, slug, summary, status, view_count, like_count FROM posts WHERE id = ? AND status = 2"
        ).bind(id).fetch_optional(pool).await.unwrap_or(None)
    };

    let post = match post {
        Some(p) => p,
        None => {
            return Json(PostDetailResp {
                code: 404,
                id: 0,
                author_id: 0,
                category_id: 0,
                title: "文章不存在".into(),
                slug: String::new(),
                summary: String::new(),
                content: String::new(),
                status: 0,
                view_count: 0,
                like_count: 0,
                is_pinned: false,
                created_at: String::new(),
                published_at: String::new(),
                tags: vec![],
                comment_count: 0,
            });
        }
    };

    let tags: Vec<String> = {
        let _s = state.tracing.span_name(&trace, "query_tags");
        sqlx::query_as::<_, (String,)>(
            "SELECT t.name FROM tags t JOIN post_tags pt ON t.id = pt.tag_id WHERE pt.post_id = ?",
        )
        .bind(id)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|t| t.0)
        .collect()
    };

    Json(PostDetailResp {
        code: 0,
        id: post.0,
        author_id: post.1,
        category_id: 0,
        title: post.2,
        slug: post.3,
        summary: post.4,
        content: String::new(),
        status: post.5,
        view_count: post.6,
        like_count: post.7,
        is_pinned: false,
        created_at: String::new(),
        published_at: String::new(),
        tags,
        comment_count: 0,
    })
}

// ── GET /api/stats ─────────────────────────────────────────

#[afast::get(desc("站点统计"))]
pub async fn site_stats(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
) -> Json<DashboardStats> {
    let pool = state.db.pool();

    let (total_users, total_posts, total_comments) = {
        let _s = state.tracing.span_name(&trace, "aggregate_stats");
        let u: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(pool)
            .await
            .unwrap_or((0,));
        let p: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts")
            .fetch_one(pool)
            .await
            .unwrap_or((0,));
        let c: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM comments")
            .fetch_one(pool)
            .await
            .unwrap_or((0,));
        (u.0, p.0, c.0)
    };

    Json(DashboardStats {
        total_posts,
        total_users,
        total_comments,
        total_categories: 0,
        total_tags: 0,
    })
}

// ── 构建服务 ────────────────────────────────────────────────

pub fn build_service() -> afast::Service {
    afast::service!("api", "HTTP REST 接口" => {
        get("/api/health", health_check),
        get("/api/categories", get_categories),
        get("/api/tags", get_tags),
        get("/api/posts", get_posts),
        get("/api/posts/:id", get_post_by_id),
        get("/api/stats", site_stats),
    })
}
