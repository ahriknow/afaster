# 静态网页服务

Feature: `serve` / `serve-embed` | 依赖: `mime_guess`

## 简介

静态网页服务模块，用于向前端提供静态资源（HTML、CSS、JS、图片、字体等）。

适用场景：将 Vue、React 等前端框架构建产物部署为后端静态资源，通过 `get("*", handler)` 实现 SPA 路由。

支持两种模式：

| 模式 | Feature | 说明 |
|------|---------|------|
| 运行时目录 | `serve` | 启动时从指定目录读取文件 |
| 编译期嵌入 | `serve-embed` | 使用 `include_dir` 将整个目录打包进二进制文件 |

## 配置

`config.toml`：

```toml
[serve]
# prefix = "/"           # URL 前缀，默认 "/"
spa = true               # SPA 模式：未找到文件时返回 index.html，默认 false
```

## 使用方式

### 运行时目录模式

将前端构建产物放在项目目录下（如 `./dist`），启动时读取：

```rust,no_run
use afaster::{AFaster, serve::Serve};

#[tokio::main]
async fn main() {
    AFaster::new("config.toml".to_string()).await
        .unwrap()
        .with_serve(Serve::from_dir("./dist").with_spa(true))
        .run()
        .await;
}
```

### 编译期嵌入模式

使用 `include_dir!` 宏在编译期将整个目录嵌入二进制文件，部署时无需携带静态资源文件：

```rust,no_run
use afaster::{AFaster, serve::Serve};

#[tokio::main]
async fn main() {
    AFaster::new("config.toml".to_string()).await
        .unwrap()
        .with_serve(
            Serve::from_embedded(include_dir!("$CARGO_MANIFEST_DIR/dist"))
                .with_spa(true)
        )
        .run()
        .await;
}
```

> **提示**：`include_dir!` 使用 `$CARGO_MANIFEST_DIR` 宏变量指向当前 crate 根目录，确保 `dist` 目录在编译时存在。

### 自定义 URL 前缀

默认从根路径 `/` 提供服务。可通过 `with_prefix()` 设置前缀：

```rust,no_run
use afaster::{AFaster, serve::Serve};

#[tokio::main]
async fn main() {
    AFaster::new("config.toml".to_string()).await
        .unwrap()
        .with_serve(
            Serve::from_dir("./dist")
                .with_prefix("/app")
                .with_spa(true)
        )
        .run()
        .await;
}
```

此时静态资源通过 `/app/*` 路径访问，如 `/app/index.html`、`/app/assets/main.js`。

## SPA 模式

启用 SPA 模式（`with_spa(true)` 或 `config.toml` 中 `spa = true`）后：

1. 请求路径匹配到文件 → 返回文件内容
2. 请求路径未匹配到文件 → 返回 `index.html`

这对 Vue Router 的 `history` 模式、React Router 等前端路由方案至关重要。

## 工作原理

1. 框架在 `run()` 阶段自动注册 `get("*", serve_handler)` 路由
2. 每个请求到达时，handler 从 `FullPath` 提取请求路径
3. 去除配置的 URL 前缀，得到相对路径
4. 从文件来源（目录或嵌入数据）查找文件
5. 根据文件扩展名推断 MIME 类型
6. 返回文件内容，浏览器自动识别类型

## 安全

- 路径遍历防护：拒绝包含 `..` 的路径
- 仅响应 GET 请求
- 自动 MIME 类型推断，防止内容嗅探攻击

## API

### Serve

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `from_dir` | `dir: impl Into<PathBuf>` | `Serve` | 从运行时目录创建 |
| `from_embedded` | `dir: include_dir::Dir` | `Serve` | 从编译期嵌入数据创建（需要 `serve-embed`） |
| `with_prefix` | `prefix: impl Into<String>` | `Serve` | 设置 URL 前缀 |
| `with_spa` | `spa: bool` | `Serve` | 启用/禁用 SPA 模式 |

### ServeConfig

```rust
#[derive(Deserialize)]
pub struct ServeConfig {
    pub prefix: Option<String>,  // URL 前缀，默认 "/"
    pub spa: bool,               // SPA 模式，默认 false
}
```

## Feature 列表

| Feature | 说明 |
|---------|------|
| `serve` | 基础静态文件服务（运行时目录） |
| `serve-embed` | 编译期嵌入模式（依赖 `include_dir`） |
