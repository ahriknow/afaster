//! 管理员接口：用户管理、文章审核、分类标签管理、角色管理
//! 使用方案B（闭包式 trace）

use crate::model::*;
use afast::{Ctx, Custom, Data, State, handler};
use afaster::trace::TraceContext;

// ── 封禁/解封 ────────────────────────────────────────────────

#[handler(desc("封禁用户"))]
async fn ban_user(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Custom(auth): Custom<AuthToken>,
    Data(req): Data<UserActionReq>,
) -> afast::Result<ApiResp> {
    let uid = state.token.get_id(&auth.token)?;

    state
        .tracing
        .trace_with(&trace, "check_permission", "检查权限", || async {
            if let Some(ref rbac) = state.rbac {
                if !rbac
                    .check_permission(uid, "user:ban")
                    .await
                    .unwrap_or(false)
                {
                    return;
                }
            }
        })
        .await;

    if let Some(ref rbac) = state.rbac {
        if !rbac
            .check_permission(uid, "user:ban")
            .await
            .unwrap_or(false)
        {
            return Ok(ApiResp::err(40301, "权限不足"));
        }
    }

    state
        .tracing
        .trace_with(&trace, "ban_user", "封禁用户", || async {
            let pool = state.db.sqlite();
            sqlx::query("UPDATE users SET status = 1, updated_at = datetime('now') WHERE id = ?")
                .bind(req.target_id)
                .execute(pool)
                .await
                .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))
        })
        .await?;

    Ok(ApiResp::ok("已封禁"))
}

#[handler(desc("解封用户"))]
async fn unban_user(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Custom(auth): Custom<AuthToken>,
    Data(req): Data<UserActionReq>,
) -> afast::Result<ApiResp> {
    let uid = state.token.get_id(&auth.token)?;

    if let Some(ref rbac) = state.rbac {
        if !rbac
            .check_permission(uid, "user:ban")
            .await
            .unwrap_or(false)
        {
            return Ok(ApiResp::err(40301, "权限不足"));
        }
    }

    state
        .tracing
        .trace_with(&trace, "unban_user", "解封用户", || async {
            let pool = state.db.sqlite();
            sqlx::query("UPDATE users SET status = 0, updated_at = datetime('now') WHERE id = ?")
                .bind(req.target_id)
                .execute(pool)
                .await
                .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))
        })
        .await?;

    Ok(ApiResp::ok("已解封"))
}

// ── 角色分配 ──────────────────────────────────────────────────

#[handler(desc("分配角色"))]
async fn assign_role(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Custom(auth): Custom<AuthToken>,
    Data(req): Data<AssignRoleReq>,
) -> afast::Result<ApiResp> {
    let uid = state.token.get_id(&auth.token)?;

    if let Some(ref rbac) = state.rbac {
        if !rbac
            .check_permission(uid, "role:assign")
            .await
            .unwrap_or(false)
        {
            return Ok(ApiResp::err(40301, "权限不足"));
        }
    }

    state
        .tracing
        .trace_with(&trace, "assign_role", "分配角色", || async {
            if let Some(ref rbac) = state.rbac {
                rbac.assign_role(req.target_id, &req.role_name).await?;
            }
            Ok::<_, afast::Error>(())
        })
        .await?;

    Ok(ApiResp::ok("角色分配成功"))
}

// ── 文章审核 ──────────────────────────────────────────────────

#[handler(desc("审核文章"))]
async fn review_post(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Custom(auth): Custom<AuthToken>,
    Data(req): Data<ReviewPostReq>,
) -> afast::Result<ApiResp> {
    let uid = state.token.get_id(&auth.token)?;

    if let Some(ref rbac) = state.rbac {
        if !rbac
            .check_permission(uid, "post:review")
            .await
            .unwrap_or(false)
        {
            return Ok(ApiResp::err(40301, "权限不足"));
        }
    }

    let new_status = match req.action.as_str() {
        "approve" => 2,
        "reject" => 3,
        _ => return Ok(ApiResp::err(40001, "无效操作")),
    };
    let pub_at = if new_status == 2 {
        Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
    } else {
        None
    };

    state.tracing.trace_with(&trace, "review_post", "审核文章", || async {
        let pool = state.db.sqlite();
        sqlx::query("UPDATE posts SET status = ?, published_at = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(new_status).bind(&pub_at).bind(req.post_id).execute(pool).await
            .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))
    }).await?;

    Ok(ApiResp::ok(if new_status == 2 {
        "已通过"
    } else {
        "已拒绝"
    }))
}

#[handler(desc("待审核文章"))]
async fn pending_posts(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Custom(auth): Custom<AuthToken>,
) -> afast::Result<PendingPostListResp> {
    let uid = state.token.get_id(&auth.token)?;

    if let Some(ref rbac) = state.rbac {
        if !rbac
            .check_permission(uid, "post:review")
            .await
            .unwrap_or(false)
        {
            return Ok(PendingPostListResp {
                code: 40301,
                data: vec![],
            });
        }
    }

    let rows = state.tracing.trace_with(&trace, "query_pending", "查询待审核", || async {
        let pool = state.db.sqlite();
        sqlx::query_as::<_, (i64, i64, String, String, String)>(
            "SELECT id, author_id, title, summary, created_at FROM posts WHERE status = 1 ORDER BY id ASC"
        ).fetch_all(pool).await.unwrap_or_default()
    }).await;

    Ok(PendingPostListResp {
        code: 0,
        data: rows
            .into_iter()
            .map(|r| PendingPostItem {
                id: r.0,
                author_id: r.1,
                title: r.2,
                summary: r.3,
                created_at: r.4,
            })
            .collect(),
    })
}

// ── 分类管理 ──────────────────────────────────────────────────

#[handler(desc("创建分类"))]
async fn create_category(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Custom(auth): Custom<AuthToken>,
    Data(req): Data<CreateCategoryReq>,
) -> afast::Result<ApiResp> {
    let uid = state.token.get_id(&auth.token)?;

    if let Some(ref rbac) = state.rbac {
        if !rbac
            .check_permission(uid, "role:create")
            .await
            .unwrap_or(false)
        {
            return Ok(ApiResp::err(40301, "权限不足"));
        }
    }

    state
        .tracing
        .trace_with(&trace, "create_category", "创建分类", || async {
            let pool = state.db.sqlite();
            sqlx::query("INSERT INTO categories (name, slug, description) VALUES (?, ?, ?)")
                .bind(&req.name)
                .bind(&req.slug)
                .bind(&req.description)
                .execute(pool)
                .await
                .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))
        })
        .await?;

    Ok(ApiResp::ok("分类创建成功"))
}

#[handler(desc("删除分类"))]
async fn delete_category(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Custom(auth): Custom<AuthToken>,
    Data(req): Data<DeleteCategoryReq>,
) -> afast::Result<ApiResp> {
    let uid = state.token.get_id(&auth.token)?;

    if let Some(ref rbac) = state.rbac {
        if !rbac
            .check_permission(uid, "role:delete")
            .await
            .unwrap_or(false)
        {
            return Ok(ApiResp::err(40301, "权限不足"));
        }
    }

    state
        .tracing
        .trace_with(&trace, "delete_category", "删除分类", || async {
            let pool = state.db.sqlite();
            sqlx::query("UPDATE posts SET category_id = NULL WHERE category_id = ?")
                .bind(req.cat_id)
                .execute(pool)
                .await
                .ok();
            sqlx::query("DELETE FROM categories WHERE id = ?")
                .bind(req.cat_id)
                .execute(pool)
                .await
                .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))
        })
        .await?;

    Ok(ApiResp::ok("分类删除成功"))
}

// ── 标签管理 ──────────────────────────────────────────────────

#[handler(desc("创建标签"))]
async fn create_tag(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Custom(auth): Custom<AuthToken>,
    Data(req): Data<CreateTagReq>,
) -> afast::Result<ApiResp> {
    let uid = state.token.get_id(&auth.token)?;

    if let Some(ref rbac) = state.rbac {
        if !rbac
            .check_permission(uid, "role:create")
            .await
            .unwrap_or(false)
        {
            return Ok(ApiResp::err(40301, "权限不足"));
        }
    }

    state
        .tracing
        .trace_with(&trace, "create_tag", "创建标签", || async {
            let pool = state.db.sqlite();
            sqlx::query("INSERT INTO tags (name, slug) VALUES (?, ?)")
                .bind(&req.name)
                .bind(&req.slug)
                .execute(pool)
                .await
                .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))
        })
        .await?;

    Ok(ApiResp::ok("标签创建成功"))
}

#[handler(desc("删除标签"))]
async fn delete_tag(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Custom(auth): Custom<AuthToken>,
    Data(req): Data<DeleteTagReq>,
) -> afast::Result<ApiResp> {
    let uid = state.token.get_id(&auth.token)?;

    if let Some(ref rbac) = state.rbac {
        if !rbac
            .check_permission(uid, "role:delete")
            .await
            .unwrap_or(false)
        {
            return Ok(ApiResp::err(40301, "权限不足"));
        }
    }

    state
        .tracing
        .trace_with(&trace, "delete_tag", "删除标签", || async {
            let pool = state.db.sqlite();
            sqlx::query("DELETE FROM post_tags WHERE tag_id = ?")
                .bind(req.tag_id)
                .execute(pool)
                .await
                .ok();
            sqlx::query("DELETE FROM tags WHERE id = ?")
                .bind(req.tag_id)
                .execute(pool)
                .await
                .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))
        })
        .await?;

    Ok(ApiResp::ok("标签删除成功"))
}

// ── 评论管理 ──────────────────────────────────────────────────

#[handler(desc("删除评论"))]
async fn delete_comment(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Custom(auth): Custom<AuthToken>,
    Data(req): Data<DeleteCommentReq>,
) -> afast::Result<ApiResp> {
    let uid = state.token.get_id(&auth.token)?;

    if let Some(ref rbac) = state.rbac {
        if !rbac
            .check_permission(uid, "comment:delete")
            .await
            .unwrap_or(false)
        {
            return Ok(ApiResp::err(40301, "权限不足"));
        }
    }

    state
        .tracing
        .trace_with(&trace, "delete_comment", "删除评论", || async {
            let pool = state.db.sqlite();
            sqlx::query("UPDATE comments SET status = 1 WHERE id = ?")
                .bind(req.comment_id)
                .execute(pool)
                .await
                .map_err(|e| afaster::Error::custom(50002, format!("DB: {}", e)))
        })
        .await?;

    Ok(ApiResp::ok("评论已删除"))
}

// ── 角色/权限列表 ────────────────────────────────────────────

#[handler(desc("角色列表"))]
async fn list_roles(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Custom(auth): Custom<AuthToken>,
) -> afast::Result<RoleListResp> {
    let uid = state.token.get_id(&auth.token)?;

    if let Some(ref rbac) = state.rbac {
        if !rbac
            .check_permission(uid, "role:list")
            .await
            .unwrap_or(false)
        {
            return Ok(RoleListResp {
                code: 40301,
                data: vec![],
            });
        }
    }

    let roles = state
        .tracing
        .trace_with(&trace, "list_roles", "查询角色", || async {
            if let Some(ref rbac) = state.rbac {
                rbac.list_roles().await.unwrap_or_default()
            } else {
                vec![]
            }
        })
        .await;

    Ok(RoleListResp {
        code: 0,
        data: roles
            .into_iter()
            .map(|r| RoleItem {
                id: r.id,
                name: r.name,
                display_name: r.display_name,
            })
            .collect(),
    })
}

#[handler(desc("权限列表"))]
async fn list_permissions(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Custom(auth): Custom<AuthToken>,
) -> afast::Result<PermissionListResp> {
    let uid = state.token.get_id(&auth.token)?;

    if let Some(ref rbac) = state.rbac {
        if !rbac
            .check_permission(uid, "role:list")
            .await
            .unwrap_or(false)
        {
            return Ok(PermissionListResp {
                code: 40301,
                data: vec![],
            });
        }
    }

    let perms = state
        .tracing
        .trace_with(&trace, "list_permissions", "查询权限", || async {
            vec![]
        })
        .await;

    Ok(PermissionListResp {
        code: 0,
        data: perms,
    })
}

pub fn build_service() -> afast::Service {
    afast::service!("admin", "管理员接口" => {
        h(ban_user),
        h(unban_user),
        h(assign_role),
        h(review_post),
        h(pending_posts),
        h(create_category),
        h(delete_category),
        h(create_tag),
        h(delete_tag),
        h(delete_comment),
        h(list_roles),
        h(list_permissions),
    })
}
