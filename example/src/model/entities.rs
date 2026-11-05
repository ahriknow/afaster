use afastorm::{FromRow, Model};

#[derive(Clone, Debug, FromRow, Model)]
#[orm(table = "users")]
pub struct User {
    #[orm(primary_key, auto_increment)]
    pub id: Option<i64>,
    #[orm(unique)]
    #[orm(find_by)]
    pub username: String,
    pub nickname: String,
    #[orm(unique)]
    pub email: String,
    pub password_hash: String,
    pub avatar: String,
    pub bio: String,
    pub status: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, FromRow, Model)]
#[orm(
    table = "categories",
    admin(label = "分类", list, detail, create, update, delete)
)]
pub struct Category {
    #[orm(primary_key, auto_increment)]
    pub id: Option<i64>,
    #[orm(unique, admin(label = "名称", list, detail, create, update, search))]
    pub name: String,
    #[orm(unique, admin(label = "Slug", list, detail, create, update, search))]
    pub slug: String,
    #[orm(admin(label = "描述", detail, create, update))]
    pub description: String,
    #[orm(admin(label = "排序", list, detail, create, update))]
    pub sort_order: i32,
    pub created_at: String,
}

#[derive(Clone, Debug, FromRow, Model)]
#[orm(
    table = "tags",
    find_by(slug),
    admin(label = "标签", list, detail, create, update, delete)
)]
pub struct Tag {
    #[orm(primary_key, auto_increment)]
    pub id: Option<i64>,
    #[orm(unique, admin(label = "名称", list, detail, create, update, search))]
    #[orm(find_by)]
    pub name: String,
    #[orm(unique, admin(label = "Slug", list, detail, create, update, search))]
    pub slug: String,
    pub created_at: String,
}

#[derive(Clone, Debug, FromRow, Model)]
#[orm(
    table = "posts",
    find_by(author_id, slug),
    find_json(metadata.featured, eq),
    find_json(metadata, contains, has_key),
    admin(label = "文章", list, detail, create, update, delete)
)]
pub struct Post {
    #[orm(primary_key, auto_increment)]
    pub id: Option<i64>,
    pub author_id: i64,
    pub category_id: Option<i64>,
    #[orm(admin(label = "标题", list, detail, create, update, search))]
    pub title: String,
    #[orm(find_by)]
    pub slug: String,
    #[orm(admin(label = "摘要", list, detail, create, update, search))]
    pub summary: String,
    #[orm(admin(label = "正文", detail, create, update))]
    pub content: String,
    pub cover_image: String,
    #[orm(admin(label = "扩展元数据", detail, create, update))]
    pub metadata: serde_json::Value,
    #[orm(admin(label = "状态", list, detail, create, update))]
    pub status: i32,
    pub view_count: i64,
    pub like_count: i64,
    pub is_pinned: bool,
    #[orm(auto_create, admin(label = "创建时间", list, detail, readonly))]
    pub created_at: String,
    #[orm(auto_update, admin(label = "更新时间", detail, readonly))]
    pub updated_at: String,
    #[orm(admin(label = "发布时间", list, detail, create, update))]
    pub published_at: Option<String>,
    #[orm(soft_delete, admin(hidden))]
    pub deleted_at: Option<i64>,
    #[orm(version, admin(label = "版本", detail, readonly))]
    pub version: i64,
}

#[derive(Clone, Debug, FromRow, Model)]
#[orm(table = "post_tags")]
pub struct PostTag {
    pub post_id: i64,
    pub tag_id: i64,
}

#[derive(Clone, Debug, FromRow, Model)]
#[orm(table = "post_likes")]
pub struct PostLike {
    pub post_id: i64,
    pub user_id: i64,
    pub created_at: String,
}

#[derive(Clone, Debug, FromRow, Model)]
#[orm(
    table = "comments",
    admin(label = "评论", list, detail, create, update, delete)
)]
pub struct Comment {
    #[orm(primary_key, auto_increment)]
    pub id: Option<i64>,
    pub post_id: i64,
    pub author_id: i64,
    pub parent_id: Option<i64>,
    #[orm(admin(label = "内容", list, detail, create, update, search))]
    pub content: String,
    #[orm(admin(label = "状态", list, detail, create, update))]
    pub status: i32,
    pub created_at: String,
}

#[derive(Clone, Debug, FromRow, Model)]
#[orm(table = "rbac_roles")]
pub struct RbacRole {
    #[orm(primary_key, auto_increment)]
    pub id: Option<i64>,
    #[orm(unique)]
    pub name: String,
    pub display_name: String,
    pub created_at: String,
}

#[derive(Clone, Debug, FromRow, Model)]
#[orm(table = "rbac_role_permissions")]
pub struct RbacRolePermission {
    pub role_id: i64,
    pub permission_code: String,
}

#[derive(Clone, Debug, FromRow, Model)]
#[orm(table = "rbac_user_roles")]
pub struct RbacUserRole {
    pub user_id: i64,
    pub role_id: i64,
}

pub fn registry() -> afastorm::ModelRegistry {
    afastorm::ModelRegistry::new()
        .register::<User>()
        .register::<Category>()
        .register::<Tag>()
        .register::<Post>()
        .register::<PostTag>()
        .register::<PostLike>()
        .register::<Comment>()
        .register::<RbacRole>()
        .register::<RbacRolePermission>()
        .register::<RbacUserRole>()
}
