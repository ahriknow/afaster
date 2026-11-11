//! 普通 HTTP REST 接口（ordinary-http）
//!
//! 使用 `#[afast::get]` / `#[afast::post]` 宏，演示 HTTP 接口的链路追踪。

use crate::model::entities::{Category, Comment, Post, PostFindJson as _, PostTag, Tag, User};
use crate::model::*;
use afast::{Ctx, Json, State};
use afaster::trace::TraceContext;
use afastorm::Field;
use serde_json::json;

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
) -> afast::Result<Json<CategoryListResp>> {
    let rows = {
        let _s = state.tracing.span(&trace);
        let repository = state.orm.repo::<Category>().map_err(db_error)?;
        let statement = repository
            .query()
            .order_by(Category::sort_order.asc())
            .build()
            .map_err(db_error)?;
        repository.fetch_all(&statement).await.map_err(db_error)?
    };

    Ok(Json(CategoryListResp {
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
    }))
}

// ── GET /api/tags ──────────────────────────────────────────

#[afast::get(desc("获取标签列表"))]
pub async fn get_tags(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
) -> afast::Result<Json<TagListResp>> {
    let rows = {
        let _s = state.tracing.span(&trace);
        let repository = state.orm.repo::<Tag>().map_err(db_error)?;
        let statement = repository
            .query()
            .order_by(Tag::id.asc())
            .build()
            .map_err(db_error)?;
        repository.fetch_all(&statement).await.map_err(db_error)?
    };

    Ok(Json(TagListResp {
        code: 0,
        data: rows
            .into_iter()
            .map(|row| TagItem {
                id: row.id.unwrap_or_default(),
                name: row.name,
                slug: row.slug,
            })
            .collect(),
    }))
}

// ── GET /api/posts ─────────────────────────────────────────

#[afast::get(desc("获取文章列表"))]
pub async fn get_posts(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
) -> afast::Result<Json<PostListResp>> {
    let repo = state.orm.repo::<Post>().map_err(db_error)?;
    let total = {
        let _s = state.tracing.span_name(&trace, "count_posts");
        repo.count_where(Field::new("status").eq(2))
            .await
            .map_err(db_error)?
    };

    let rows = {
        let _s = state.tracing.span_name(&trace, "query_posts");
        let statement = repo
            .query()
            .filter(Field::new("status").eq(2))
            .order_by(Post::published_at.desc())
            .limit(50)
            .build()
            .map_err(db_error)?;
        repo.fetch_all(&statement).await.map_err(db_error)?
    };

    Ok(Json(PostListResp {
        code: 0,
        total,
        page: 1,
        page_size: 50,
        data: rows
            .into_iter()
            .map(|row| PostItem {
                id: row.id.unwrap_or_default(),
                author_id: row.author_id,
                category_id: row.category_id.unwrap_or_default(),
                title: row.title,
                summary: row.summary,
                status: row.status,
                view_count: row.view_count,
                like_count: row.like_count,
                created_at: crate::model::entities::format_datetime(row.created_at),
                published_at: crate::model::entities::format_optional_datetime(row.published_at),
            })
            .collect(),
    }))
}

// ── GET /api/posts/featured ─────────────────────────────────

#[afast::get(desc("获取精选文章"))]
pub async fn get_featured_post(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
) -> afast::Result<Json<PostListResp>> {
    let post = {
        let _s = state.tracing.span_name(&trace, "query_featured_post");
        state
            .orm
            .repo::<Post>()
            .map_err(db_error)?
            .find_json_eq_metadata_featured(json!(true))
            .await
            .map_err(db_error)?
    };
    let data = post
        .map(|row| {
            vec![PostItem {
                id: row.id.unwrap_or_default(),
                author_id: row.author_id,
                category_id: row.category_id.unwrap_or_default(),
                title: row.title,
                summary: row.summary,
                status: row.status,
                view_count: row.view_count,
                like_count: row.like_count,
                created_at: crate::model::entities::format_datetime(row.created_at),
                published_at: crate::model::entities::format_optional_datetime(row.published_at),
            }]
        })
        .unwrap_or_default();

    Ok(Json(PostListResp {
        code: 0,
        total: data.len() as i64,
        page: 1,
        page_size: 1,
        data,
    }))
}

// ── GET /api/posts/:id ─────────────────────────────────────

#[afast::get(desc("获取文章详情"))]
pub async fn get_post_by_id(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    afast::Query(params): afast::Query<PostIdReq>,
) -> afast::Result<Json<PostDetailResp>> {
    let id = params.post_id;

    let post = {
        let _s = state.tracing.span_name(&trace, "query_post");
        state
            .orm
            .repo::<Post>()
            .map_err(db_error)?
            .first_where(Field::new("id").eq(id).and(Field::new("status").eq(2)))
            .await
            .map_err(db_error)?
    };

    let post = match post {
        Some(post) => post,
        None => {
            return Ok(Json(PostDetailResp {
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
            }));
        }
    };

    let _s = state.tracing.span_name(&trace, "query_tags");
    let post_id = post.id.unwrap_or_default();
    let tags = {
        let links_statement = state
            .orm
            .repo::<PostTag>()
            .map_err(db_error)?
            .query()
            .filter(Field::new("post_id").eq(post_id))
            .build()
            .map_err(db_error)?;
        let links = state
            .orm
            .fetch_all::<PostTag>(&links_statement)
            .await
            .map_err(db_error)?;
        let tag_repository = state.orm.repo::<Tag>().map_err(db_error)?;
        let mut names = Vec::with_capacity(links.len());
        for link in links {
            if let Some(tag) = tag_repository
                .find_by_id(link.tag_id)
                .await
                .map_err(db_error)?
            {
                names.push(tag.name);
            }
        }
        names
    };
    let comment_count = state
        .orm
        .repo::<Comment>()
        .map_err(db_error)?
        .count_where(Field::new("post_id").eq(post_id))
        .await
        .map_err(db_error)?;

    Ok(Json(PostDetailResp {
        code: 0,
        id: post_id,
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
        created_at: crate::model::entities::format_datetime(post.created_at),
        published_at: crate::model::entities::format_optional_datetime(post.published_at),
        tags,
        comment_count,
    }))
}

// ── GET /api/stats ─────────────────────────────────────────

#[afast::get(desc("站点统计"))]
pub async fn site_stats(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
) -> afast::Result<Json<DashboardStats>> {
    let (total_users, total_posts, total_comments) = {
        let _s = state.tracing.span_name(&trace, "aggregate_stats");
        let users = state
            .orm
            .repo::<User>()
            .map_err(db_error)?
            .count()
            .await
            .map_err(db_error)?;
        let posts = state
            .orm
            .repo::<Post>()
            .map_err(db_error)?
            .count()
            .await
            .map_err(db_error)?;
        let comments = state
            .orm
            .repo::<Comment>()
            .map_err(db_error)?
            .count()
            .await
            .map_err(db_error)?;
        (users, posts, comments)
    };

    Ok(Json(DashboardStats {
        total_posts,
        total_users,
        total_comments,
        total_categories: 0,
        total_tags: 0,
    }))
}

fn db_error(error: impl std::fmt::Display) -> afast::Error {
    afaster::Error::custom(50002, format!("DB: {error}")).into()
}

// ── 构建服务 ────────────────────────────────────────────────

pub fn build_service() -> afast::Service {
    afast::service!("api", "HTTP REST 接口" => {
        get("/api/health", health_check),
        get("/api/categories", get_categories),
        get("/api/tags", get_tags),
        get("/api/posts", get_posts),
        get("/api/posts/featured", get_featured_post),
        get("/api/posts/:id", get_post_by_id),
        get("/api/stats", site_stats),
    })
}
