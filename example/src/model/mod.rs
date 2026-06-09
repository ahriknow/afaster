//! 数据模型 — 所有请求/响应类型均 derive AFastSerialize + AFastDeserialize + Tag
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════
//  通用
// ═══════════════════════════════════════════════════════════════

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("通用响应")]
pub struct ApiResp {
    pub code: i32,
    pub message: String,
}

impl ApiResp {
    pub fn ok(msg: &str) -> Self {
        Self {
            code: 0,
            message: msg.into(),
        }
    }
    pub fn err(code: i32, msg: &str) -> Self {
        Self {
            code,
            message: msg.into(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  认证上下文（客户端连接时设置，所有请求自动携带）
// ═══════════════════════════════════════════════════════════════

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("认证令牌")]
pub struct AuthToken {
    pub token: String,
}

// ═══════════════════════════════════════════════════════════════
//  用户认证
// ═══════════════════════════════════════════════════════════════

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("注册请求")]
pub struct RegisterReq {
    pub username: String,
    pub nickname: String,
    pub email: String,
    pub password: String,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("登录请求")]
pub struct LoginReq {
    pub username: String,
    pub password: String,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("登录响应")]
pub struct LoginResp {
    pub code: i32,
    pub message: String,
    pub token: String,
    pub user_id: i64,
    pub username: String,
    pub nickname: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

// ═══════════════════════════════════════════════════════════════
//  带 Token 的请求（后台接口共用）
// ═══════════════════════════════════════════════════════════════

/// 分页请求
#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("分页请求")]
pub struct PageReq {
    pub page: i32,
}

/// 目标用户 ID
#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("用户操作请求")]
pub struct UserActionReq {
    pub target_id: i64,
}

/// 角色分配
#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("角色分配请求")]
pub struct AssignRoleReq {
    pub target_id: i64,
    pub role_name: String,
}

// ═══════════════════════════════════════════════════════════════
//  文章
// ═══════════════════════════════════════════════════════════════

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("文章列表请求")]
pub struct ListPostsReq {
    pub page: i32,
    pub page_size: i32,
    pub category_id: i64,
    pub keyword: String,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("文章ID请求")]
pub struct PostIdReq {
    pub post_id: i64,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("创建文章请求")]
pub struct CreatePostReq {
    pub title: String,
    pub content: String,
    pub summary: String,
    pub category_id: i64,
    pub tags: Vec<String>,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("文章详情响应")]
pub struct PostDetailResp {
    pub code: i32,
    pub id: i64,
    pub author_id: i64,
    pub category_id: i64,
    pub title: String,
    pub slug: String,
    pub summary: String,
    pub content: String,
    pub status: i32,
    pub view_count: i64,
    pub like_count: i64,
    pub is_pinned: bool,
    pub created_at: String,
    pub published_at: String,
    pub tags: Vec<String>,
    pub comment_count: i64,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("文章列表项")]
pub struct PostItem {
    pub id: i64,
    pub author_id: i64,
    pub category_id: i64,
    pub title: String,
    pub summary: String,
    pub status: i32,
    pub view_count: i64,
    pub like_count: i64,
    pub created_at: String,
    pub published_at: String,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("文章列表响应")]
pub struct PostListResp {
    pub code: i32,
    pub total: i64,
    pub page: i32,
    pub page_size: i32,
    pub data: Vec<PostItem>,
}

// ═══════════════════════════════════════════════════════════════
//  评论
// ═══════════════════════════════════════════════════════════════

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("创建评论请求")]
pub struct CreateCommentReq {
    pub post_id: i64,
    pub content: String,
    pub parent_id: i64,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("删除评论请求")]
pub struct DeleteCommentReq {
    pub comment_id: i64,
}

// ═══════════════════════════════════════════════════════════════
//  分类 & 标签
// ═══════════════════════════════════════════════════════════════

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("创建分类请求")]
pub struct CreateCategoryReq {
    pub name: String,
    pub slug: String,
    pub description: String,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("删除分类请求")]
pub struct DeleteCategoryReq {
    pub cat_id: i64,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("创建标签请求")]
pub struct CreateTagReq {
    pub name: String,
    pub slug: String,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("删除标签请求")]
pub struct DeleteTagReq {
    pub tag_id: i64,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("分类项")]
pub struct CategoryItem {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub sort_order: i32,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("分类列表响应")]
pub struct CategoryListResp {
    pub code: i32,
    pub data: Vec<CategoryItem>,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("标签项")]
pub struct TagItem {
    pub id: i64,
    pub name: String,
    pub slug: String,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("标签列表响应")]
pub struct TagListResp {
    pub code: i32,
    pub data: Vec<TagItem>,
}

// ═══════════════════════════════════════════════════════════════
//  审核
// ═══════════════════════════════════════════════════════════════

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("审核文章请求")]
pub struct ReviewPostReq {
    pub post_id: i64,
    pub action: String, // "approve" | "reject"
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("待审核文章项")]
pub struct PendingPostItem {
    pub id: i64,
    pub author_id: i64,
    pub title: String,
    pub summary: String,
    pub created_at: String,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("待审核文章列表")]
pub struct PendingPostListResp {
    pub code: i32,
    pub data: Vec<PendingPostItem>,
}

// ═══════════════════════════════════════════════════════════════
//  角色 & 权限
// ═══════════════════════════════════════════════════════════════

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("角色项")]
pub struct RoleItem {
    pub id: i64,
    pub name: String,
    pub display_name: String,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("角色列表响应")]
pub struct RoleListResp {
    pub code: i32,
    pub data: Vec<RoleItem>,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("权限项")]
pub struct PermissionItem {
    pub code: String,
    pub name: String,
    pub module: String,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("权限列表响应")]
pub struct PermissionListResp {
    pub code: i32,
    pub data: Vec<PermissionItem>,
}

// ═══════════════════════════════════════════════════════════════
//  仪表盘（复杂链路测试）
// ═══════════════════════════════════════════════════════════════

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("仪表盘请求")]
pub struct DashboardReq {
    pub user_id: i64,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("仪表盘响应")]
pub struct DashboardResp {
    pub code: i32,
    pub user: Option<DashboardUser>,
    pub recent_posts: Vec<PostItem>,
    pub categories: Vec<CategoryItem>,
    pub tags: Vec<TagItem>,
    pub stats: DashboardStats,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("仪表盘用户")]
pub struct DashboardUser {
    pub id: i64,
    pub username: String,
    pub nickname: String,
    pub post_count: i64,
    pub comment_count: i64,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("仪表盘统计")]
pub struct DashboardStats {
    pub total_posts: i64,
    pub total_users: i64,
    pub total_comments: i64,
    pub total_categories: i64,
    pub total_tags: i64,
}

// ═══════════════════════════════════════════════════════════════
//  数据报告（深度嵌套链路测试）
// ═══════════════════════════════════════════════════════════════

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("数据报告响应")]
pub struct DataReportResp {
    pub code: i32,
    pub total_users: i64,
    pub total_posts: i64,
    pub total_comments: i64,
    pub users: Vec<DashboardUser>,
    pub top_posts: Vec<PostItem>,
    pub summary: String,
    pub avg_views: f64,
    pub avg_likes: f64,
    pub engagement_rate: f64,
}

// ═══════════════════════════════════════════════════════════════
//  聊天（WS / Binary 长连接测试）
// ═══════════════════════════════════════════════════════════════

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("加入聊天室")]
pub struct ChatJoin {
    pub room: String,
    pub nickname: String,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("聊天消息")]
pub struct ChatMessage {
    pub room: String,
    pub from: String,
    pub content: String,
    pub timestamp: i64,
}
