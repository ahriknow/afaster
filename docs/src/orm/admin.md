# 自动管理端

> Feature：`orm-admin`，且需启用 `orm-sqlite` 或 `orm-postgres`

AFaster 的 `orm-admin` 集成根据注册模型的 `admin` 属性生成页面和数据 API。`afastorm` 本身只保存模型元数据，不内置管理页面；管理端也不会自动开放所有模型，只有显式配置 `admin(...)` 的模型会出现在页面中。

## 启用与保护

在 ORM 配置中设置秘密 token 和管理端路径：

```toml
[orm-sqlite]
path = "data/app.db"
token = "replace-with-a-long-random-secret"
url = "/admin"
```

在应用构造时注册模型并开启管理端：

```rust,ignore
.with_orm(|orm| {
    orm.with_models(model_registry())
        .with_admin(true)
})
```

访问配置的 URL（默认为 `/admin`），页面会要求输入 Bearer token。错误凭据会触发重新输入。请使用强随机 token，通过环境变量或部署密钥管理系统注入配置，不要使用示例密码，也不要将管理端暴露到不可信网络。

## 配置模型与字段

```rust,ignore
#[derive(Clone, Debug, FromRow, Model)]
#[orm(
    table = "posts",
    admin(label = "文章", list, detail, create, update, delete)
)]
pub struct Post {
    #[orm(primary_key, auto_increment)]
    pub id: Option<i64>,
    #[orm(admin(label = "标题", list, detail, create, update, search))]
    pub title: String,
    #[orm(admin(label = "正文", detail, create, update))]
    pub content: String,
    #[orm(auto_create, admin(label = "创建时间", list, detail, readonly))]
    pub created_at: String,
    #[orm(soft_delete, admin(hidden))]
    pub deleted_at: Option<i64>,
    #[orm(version, admin(label = "版本", detail, readonly))]
    pub version: i64,
}
```

### 模型级属性

| 属性 | 效果 |
|---|---|
| `label = "..."` | 模型在首页和标题中的名称 |
| `list` | 开启列表页 |
| `detail` | 开启详情展示 |
| `create` | 允许创建 |
| `update` | 允许更新 |
| `delete` | 允许删除 |

### 字段级属性

| 属性 | 效果 |
|---|---|
| `label = "..."` | 字段显示名称 |
| `list` | 在列表中显示 |
| `detail` | 在详情中显示 |
| `create` | 创建表单可编辑 |
| `update` | 编辑表单可编辑 |
| `search` | 参与列表文本搜索（当前仅对字符串字段搜索） |
| `readonly` | 只读字段 |
| `hidden` | 不向管理端展示 |

未标注字段默认不会显示。主键、软删除字段、版本字段及自动维护字段由管理端按元数据处理；不要把密码哈希、访问 token 或其他敏感字段配置为可见或可编辑。

## 页面功能

- 首页列出启用管理端的模型；进入模型后使用独立的 hash URL，可复制链接定位到指定模型。
- 列表支持搜索、分页和直接跳转页码。
- 创建、编辑和详情通过弹窗展示；删除前会要求二次确认。
- 操作结果以自动关闭的提示显示，悬停提示可暂停计时。
- 模型启用 `soft_delete` 后，管理端删除会执行软删除；启用 `version` 后，更新和删除会进行乐观锁校验。

管理端只是基于 bearer token 的轻量内部工具，并不替代用户身份认证、权限分层、审计日志、CSRF 防护或生产管理后台的安全设计。建议置于 VPN、私有网络或反向代理访问控制之后，并限制 token 的传播与访问范围。
