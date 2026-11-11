//! 用户后台：管理自己的文章、评论、点赞

use crate::model::entities::{Comment, Post, PostFindBy as _, PostLike, PostTag, Tag, User};
use crate::model::*;
use afast::{Ctx, Custom, Data, State, handler};
use afaster::trace::TraceContext;
use afastorm::Field;

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

    let user = {
        let _s = state.tracing.span_with(&trace, "query_user", "查询用户");
        state
            .orm
            .repo::<User>()
            .map_err(db_error)?
            .find_by_id(uid)
            .await
            .map_err(db_error)?
            .ok_or_else(|| afaster::Error::custom(40001, "用户不存在"))?
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
        user_id: user.id.unwrap_or_default(),
        username: user.username,
        nickname: user.nickname,
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
    let posts = state.orm.repo::<Post>().map_err(db_error)?;
    if posts
        .find_by_author_id_slug(uid, slug.clone())
        .await
        .map_err(db_error)?
        .is_some()
    {
        return Ok(ApiResp::err(40001, "文章别名已存在"));
    }

    let can_publish = if let Some(ref rbac) = state.rbac {
        rbac.check_permission(uid, "post:publish")
            .await
            .unwrap_or(false)
    } else {
        true
    };
    let status: i32 = if can_publish { 2 } else { 1 };
    let published_at = if status == 2 {
        Some(chrono::Local::now().naive_local())
    } else {
        None
    };

    let tags = req.tags.clone();
    {
        let _s = state
            .tracing
            .span_with(&trace, "create_post_transaction", "事务创建文章与标签");
        let mut post = Post {
            id: None,
            author_id: uid,
            category_id: (req.category_id > 0).then_some(req.category_id),
            title: req.title.clone(),
            slug,
            summary,
            content: req.content.clone(),
            cover_image: String::new(),
            metadata: serde_json::json!({
                "featured": false,
                "topics": tags,
            }),
            status,
            view_count: 0,
            like_count: 0,
            is_pinned: false,
            created_at: chrono::NaiveDateTime::default(),
            updated_at: chrono::NaiveDateTime::default(),
            published_at,
            deleted_at: None,
            version: 0,
        };
        state
            .orm
            .inner()
            .transaction(async |transaction| {
                let post_id = {
                    let mut repository = transaction.repo::<Post>()?;
                    repository.insert(&mut post).await?;
                    post.id.ok_or_else(|| {
                        afastorm::Error::Query("created post has no generated ID".to_owned())
                    })?
                };

                for tag_name in &tags {
                    let tag = {
                        let mut repository = transaction.repo::<Tag>()?;
                        let statement = repository
                            .query()
                            .filter(Field::new("name").eq(tag_name.clone()))
                            .limit(1)
                            .build()?;
                        repository.fetch_all(&statement).await?.into_iter().next()
                    };
                    let tag = if let Some(tag) = tag {
                        tag
                    } else {
                        let slug = tag_name
                            .chars()
                            .filter(|c| c.is_alphanumeric() || *c == ' ')
                            .collect::<String>()
                            .replace(' ', "-")
                            .to_lowercase();
                        let mut tag = Tag {
                            id: None,
                            name: tag_name.clone(),
                            slug,
                            created_at: chrono::Local::now().naive_local(),
                        };
                        transaction.repo::<Tag>()?.insert(&mut tag).await?;
                        tag
                    };
                    let tag_id = tag.id.ok_or_else(|| {
                        afastorm::Error::Query("tag record has no primary key".to_owned())
                    })?;
                    let link_exists = {
                        let mut repository = transaction.repo::<PostTag>()?;
                        repository
                            .first_where(
                                Field::new("post_id")
                                    .eq(post_id)
                                    .and(Field::new("tag_id").eq(tag_id)),
                            )
                            .await?
                            .is_some()
                    };
                    if !link_exists {
                        transaction
                            .repo::<PostTag>()?
                            .insert(&mut PostTag { post_id, tag_id })
                            .await?;
                    }
                }

                Ok(post_id)
            })
            .await
            .map_err(db_error)?
    };

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
    let page = req.page.max(1);
    let ps = 20i32;
    let offset = (page - 1) * ps;

    let post_repository = state.orm.repo::<Post>().map_err(db_error)?;
    let condition = Field::new("author_id")
        .eq(uid)
        .and(Field::new("status").ne(3));
    let total = {
        let _s = state.tracing.span_with(&trace, "count_posts", "统计文章");
        post_repository
            .count_where(condition.clone())
            .await
            .map_err(db_error)?
    };

    let rows = {
        let _s = state.tracing.span_with(&trace, "query_posts", "查询文章");
        let statement = post_repository
            .query()
            .filter(condition)
            .order_by(Post::id.desc())
            .limit(ps as u64)
            .offset(offset as u64)
            .build()
            .map_err(db_error)?;
        post_repository
            .fetch_all(&statement)
            .await
            .map_err(db_error)?
    };

    Ok(PostListResp {
        code: 0,
        total,
        page,
        page_size: ps,
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

    {
        let _s = state.tracing.span_with(&trace, "check_owner", "检查权限");
        let post = state
            .orm
            .repo::<Post>()
            .map_err(db_error)?
            .find_by_id(post_id.post_id)
            .await
            .map_err(db_error)?
            .ok_or_else(|| afaster::Error::custom(40001, "文章不存在"))?;
        if post.author_id != uid {
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
        let post = state
            .orm
            .repo::<Post>()
            .map_err(db_error)?
            .find_by_id(post_id.post_id)
            .await
            .map_err(db_error)?
            .ok_or_else(|| afaster::Error::custom(40001, "文章不存在"))?;
        state
            .orm
            .repo::<Post>()
            .map_err(db_error)?
            .delete_versioned(post_id.post_id, post.version)
            .await
            .map_err(db_error)?;
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

    {
        let _s = state.tracing.span_with(&trace, "check_post", "检查文章");
        let post = state
            .orm
            .repo::<Post>()
            .map_err(db_error)?
            .find_by_id(req.post_id)
            .await
            .map_err(db_error)?
            .ok_or_else(|| afaster::Error::custom(40001, "文章不存在"))?;
        if post.status != 2 {
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
        let now = chrono::Local::now().naive_local();
        let mut comment = Comment {
            id: None,
            post_id: req.post_id,
            author_id: uid,
            parent_id: parent,
            content: req.content.clone(),
            status: 0,
            created_at: now,
        };
        state
            .orm
            .repo::<Comment>()
            .map_err(db_error)?
            .insert(&mut comment)
            .await
            .map_err(db_error)?;
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

    let like_repository = state.orm.repo::<PostLike>().map_err(db_error)?;
    let existing = {
        let _s = state.tracing.span_with(&trace, "check_liked", "检查点赞");
        state
            .orm
            .repo::<PostLike>()
            .map_err(db_error)?
            .first_where(
                Field::new("post_id")
                    .eq(post_id.post_id)
                    .and(Field::new("user_id").eq(uid)),
            )
            .await
            .map_err(db_error)?
    };

    if existing.is_some() {
        let _s = state.tracing.span_with(&trace, "unlike", "取消点赞");
        like_repository
            .delete_where(
                Field::new("post_id")
                    .eq(post_id.post_id)
                    .and(Field::new("user_id").eq(uid)),
            )
            .await
            .map_err(db_error)?;
        state
            .orm
            .query("UPDATE posts SET like_count = like_count - 1 WHERE id = ?")
            .bind(post_id.post_id)
            .execute()
            .await
            .map_err(db_error)?;
        Ok(ApiResp::ok("取消点赞"))
    } else {
        let _s = state.tracing.span_with(&trace, "like", "点赞");
        let mut like = PostLike {
            post_id: post_id.post_id,
            user_id: uid,
            created_at: chrono::Local::now().naive_local(),
        };
        like_repository.insert(&mut like).await.map_err(db_error)?;
        state
            .orm
            .query("UPDATE posts SET like_count = like_count + 1 WHERE id = ?")
            .bind(post_id.post_id)
            .execute()
            .await
            .map_err(db_error)?;
        Ok(ApiResp::ok("点赞成功"))
    }
}

fn db_error(error: impl std::fmt::Display) -> afast::Error {
    afaster::Error::custom(50002, format!("DB: {error}")).into()
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
