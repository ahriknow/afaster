# 自动管理端

> Feature：`orm-admin`，且需启用 `orm-sqlite` 或 `orm-postgres`。管理端元数据由 AFaster 提供，不属于独立 ORM crate `afastorm`。

AFaster 根据 `#[derive(AFasterAdmin)]` 生成的展示元数据，以及 ORM 模型元数据创建管理页面和数据 API。数据库字段类型、表名、主键、软删除和版本锁由 `afastorm::Model` 定义；label、字段顺序、展示和编辑权限由 AFaster 管理。

`AFasterAdmin` derive 随 `orm-admin` feature 启用，并从 `afaster::orm` 重导出，应用不需要直接依赖 `afaster-derive`。该 derive 仅适用于具名字段的 struct；无效属性、空 label 或非法 `order` 会产生编译错误。模型必须同时注册到 ORM `ModelRegistry` 和管理端 `AdminModelRegistry`。

模型字段使用 `chrono` 或 `time` 类型时，分别启用 AFaster 的 `orm-chrono` / `orm-time` feature，并在 `afastorm` 依赖中启用 `chrono` / `time` feature。

## 配置与模型注册

使用 `#[admin(...)]` 声明需要开放管理端的模型和字段。模型必须同时加入 ORM `ModelRegistry` 和管理端 `AdminModelRegistry`：

```rust,ignore
use afaster::orm::{AdminModelRegistry, AFasterAdmin};
use afastorm::{FromRow, Model};
use chrono::NaiveDateTime;

#[derive(Clone, Debug, FromRow, Model, AFasterAdmin)]
#[orm(table = "posts")]
#[admin(label_lang = "model.post", list, detail, create, update, delete)]
pub struct Post {
    #[orm(primary_key, auto_increment)]
    pub id: Option<i64>,
    #[admin(label_lang = "post.title", list, order = 1, detail, create, update, search)]
    pub title: String,
    #[admin(label = "发布时间", list, order = 2, detail, create, update)]
    pub published_at: Option<NaiveDateTime>,
    #[orm(auto_create)]
    #[admin(label = "创建时间", list, detail, readonly)]
    pub created_at: NaiveDateTime,
    #[orm(auto_update)]
    #[admin(label = "更新时间", detail, readonly)]
    pub updated_at: NaiveDateTime,
    #[orm(soft_delete)]
    #[admin(hidden)]
    pub deleted_at: Option<NaiveDateTime>,
    #[orm(version)]
    #[admin(label = "版本", detail, readonly)]
    pub version: i64,
}

let app = app.with_orm(|orm| {
    orm.with_models(afastorm::ModelRegistry::new().register::<Post>())
        .with_admin(true)
        .with_admin_models(AdminModelRegistry::new().register::<Post>())
}).expect("configure ORM");
```

`.with_admin_models(...)` 只注册启用了模型级管理端属性的模型。配置的管理端模型若没有同时注册到 ORM，将在应用配置阶段报错。`AFasterAdmin` derive 在 AFaster crate 中；`#[orm(...)]` 仅保留 ORM 数据库语义。
模型级 `#[admin(...)]` 用于启用管理端模型并声明模型操作；即使字段有 `#[admin(...)]`，模型本身也必须带模型级属性才会启用。管理模型需要有效主键，列表操作还需要至少一个列表可见字段。

## `#[admin(...)]` 属性

模型属性用于启用模型及其可用操作：

| 属性 | 效果 |
|---|---|
| `label = "..."` | 模型显示名 |
| `label_lang = "key"` | 从当前语言的 i18n 翻译中读取显示名 |
| `list` / `detail` | 开启列表或详情 |
| `create` / `update` / `delete` | 允许创建、更新或删除 |

字段属性控制页面列和表单：

| 属性 | 效果 |
|---|---|
| `label = "..."` | 字段显示名；优先级高于 `label_lang` |
| `label_lang = "key"` | 从当前语言的 i18n 表中读取字段显示名 |
| `order = N` | 列表列顺序；`N` 为 `u32`，越小越靠前，未指定者排在其后 |
| `list` / `detail` | 在列表或详情中显示 |
| `create` / `update` | 出现在创建或更新表单 |
| `search` | 参与字符串字段的列表搜索 |
| `filter` | 将字段标记为筛选元数据；当前内置页面尚未提供筛选控件 |
| `readonly` | 只读字段 |
| `hidden` | 不向管理端展示 |

不指定字段 `label` 或 `label_lang` 时使用 Rust 字段名；若指定的 `label_lang` 在当前语言和默认语言中均不存在，也回退到字段名。模型同样支持 `label_lang`，缺少翻译时回退到数据库表名。属性类型、必填值和 `order` 范围在编译期校验。

模型级操作权限以及字段级读写权限由服务端 API 检查，不只是隐藏前端按钮。`search` 只对字符串类型字段生效；列表搜索通过包含匹配查询这些字段。当前页面支持分页和顺序展示，但不提供 `filter` 对应的筛选控件。

软删除字段使用 `#[orm(soft_delete)]`；版本字段使用 `#[orm(version)]`。管理端删除在有软删除字段时标记删除，否则物理删除。带版本字段的更新和删除要求提交预期版本，更新版本冲突时返回冲突错误；管理端版本字段应使用 `i64`。自动创建/更新时间字段由数据库表达式填充，版本字段创建时初始化为 `1`。

## 有序 i18n 与 URL 语言

通过 `with_i18ns` 传入按顺序排列的语言代码和翻译 map。由于 `HashMap` 本身没有顺序，最外层使用 `Vec`；第一项作为默认语言：

```rust,ignore
use std::collections::HashMap;

.with_orm(|orm| {
    orm.with_models(model_registry())
        .with_admin(true)
        .with_admin_models(admin_model_registry())
        .with_i18ns(vec![
            ("zh-CN".to_owned(), HashMap::from([
                ("model.post".to_owned(), "文章".to_owned()),
                ("post.title".to_owned(), "标题".to_owned()),
            ])),
            ("en".to_owned(), HashMap::from([
                ("model.post".to_owned(), "Posts".to_owned()),
                ("post.title".to_owned(), "Title".to_owned()),
            ])),
        ])
})
```

每个翻译 map 既用于管理端界面文本，也用于模型和字段的 `label_lang`。管理页将翻译数据直接嵌入 HTML，不提供单独的 i18n handler。语言代码必须非空且唯一；空列表会使用默认 `zh-CN` 语言。语言选择通过 URL 查询参数 `?lang=语言代码` 保存；没有该参数或代码不匹配时使用第一项。切换语言会更新 URL 并刷新页面。

## 认证、品牌与访问

访问管理端 URL（默认为 `/admin`）。静态 token 可在配置 section 中提供，也可调用 `.with_admin_token(...)`；省略或留空时不要求认证。自定义凭据与 token 验证使用成对的 `.with_admin_token_functions(generate, verify)`。静态 token、自定义认证和无认证模式详见下方示例配置及 [接入与配置](./setup.md)。

| 模式 | 配置 | 行为 |
|---|---|---|
| 无认证 | 不配置 token | 管理 API 不验证请求者；只适用于隔离的可信网络 |
| 静态 token | `[orm-sqlite]` / `[orm-postgres]` 的 `token` 或 `.with_admin_token(...)` | 浏览器提示输入已配置的 token，并以 Bearer token 访问 API |
| 自定义认证 | `.with_admin_token_functions(generate, verify)` | 登录凭据交给 `generate`；返回 token 后，后续请求由 `verify` 验证 |

自定义认证的 `POST /api/token` 用于凭据换 token，本身不要求已有 Bearer token；静态 token 模式不使用此登录接口。

管理页面本身返回 HTML；数据 API 要求 Bearer token（无认证模式除外）。自定义模式的 token 会保存在浏览器 `localStorage`，静态 token 会保存在 `sessionStorage`。请启用 HTTPS，保护凭据，并谨慎设置自定义 token 的有效期及撤销策略。管理端是通用数据维护工具，不替代业务授权、审计或审批流程。

```rust,ignore
.with_orm(|orm| {
    orm.with_models(model_registry())
        .with_admin(true)
        .with_admin_url("/backoffice")
        .with_admin_logo_letter("B")
        .with_admin_brand_text("博客管理")
        .with_admin_token_functions(
            |credential| verify_credential(credential).then(|| issue_token()),
            |token| verify_token(token),
        )
})
```

默认 logo 字母为 `A`，品牌名为 `Afast ORM`。自定义 token 生成器接收单个凭据字符串并返回 `Option<String>`；验证器接收 Bearer token 并返回 `bool`。如使用静态 token 或自定义认证，请通过安全的密钥注入机制配置凭据；无认证模式仅适用于受信任的隔离环境。

`generate` 和 `verify` 必须成对配置。`generate` 返回 `None` 或空 token 时登录失败；成功返回的 token 之后会交给 `verify` 验证。`generate` 函数应自行验证登录凭据并签发有时效、可撤销的随机 token，不能仅把用户提交的凭据原样当作 bearer token。

## 管理 API 与页面

管理服务挂载在配置的 URL（默认为 `/admin`）下，URL 必须是 path，不能含空白字符、query 或 fragment。主要路由如下：

| 方法 | 路径 | 用途 |
|---|---|---|
| `GET` | `/` | 管理页面 |
| `GET` | `/api/models` | 获取已注册模型及字段元数据 |
| `POST` | `/api/token` | 自定义认证模式下使用凭据换取 token |
| `GET` | `/api/:model` | 列表、字符串搜索及分页 |
| `POST` | `/api/:model` | 创建记录 |
| `GET` | `/api/:model/:id` | 查看记录 |
| `PATCH` | `/api/:model/:id` | 更新记录 |
| `DELETE` | `/api/:model/:id` | 删除记录 |

API 的模型标识可使用数据库表名或 Rust 模型名。请求字段仍受注册的模型权限和字段属性限制。内置页面支持布尔值、整数、浮点数、字符串、日期/时间、JSON 和字节值等编辑类型；其他数据库类型可能可以被 ORM 使用，但管理端不一定支持编辑。JSON 会在提交前校验，nullable 字段可提交 `null`。管理端不自动处理模型关联、上传文件或业务级数据校验，应由应用服务负责这些业务流程。

- 首页展示注册的管理端模型；每个模型有独立的 hash URL。
- 列表支持字段顺序、字符串搜索、分页及直接跳转。
- 新建、编辑和详情通过弹窗展示字段类型；只显示相应操作允许的字段。关闭按钮、取消按钮和 Escape 键可关闭弹窗；点击遮罩层不会关闭。
- `chrono::NaiveDate`、`chrono::NaiveDateTime` 等字段使用日期或日期时间输入控件；JSON 在提交前校验。
- nullable 字段提供 `NULL` 复选框；选中时提交 `null`，取消后输入具体值。
- 删除前二次确认，操作结果以自动关闭提示展示。
