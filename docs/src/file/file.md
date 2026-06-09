# 本地文件服务

Feature: `file` | 无外部依赖

## 简介

本地文件服务，提供文件上传、下载、删除、目录列表、URL 生成等功能。

支持文件类型校验、大小限制、路径安全检查（防路径遍历攻击）。

## 配置

```toml
[file]
root = "./uploads"                    # 存储根目录，不存在时自动创建
max_size = 10485760                   # 最大文件大小（字节），默认 10MB
allowed = ["jpg","png","gif","pdf"]   # 允许的扩展名，为空则不限制
url_prefix = "/files"                 # 访问 URL 前缀
```

## API

### FileService

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `new` | `config: &FileConfig` | `Result<FileService>` | 创建服务实例 |
| `upload` | `path: &str, data: &[u8]` | `Result<FileInfo>` | 上传文件 |
| `download` | `path: &str` | `Result<Vec<u8>>` | 下载文件 |
| `delete` | `path: &str` | `Result<()>` | 删除文件或目录 |
| `info` | `path: &str` | `Result<FileInfo>` | 获取文件信息 |
| `exists` | `path: &str` | `bool` | 检查文件是否存在 |
| `list` | `dir: &str` | `Result<DirList>` | 列出目录内容 |
| `url` | `path: &str` | `Result<String>` | 生成访问 URL |

### FileInfo

```rust
pub struct FileInfo {
    pub path: String,    // 相对路径
    pub name: String,    // 文件名
    pub ext: String,     // 扩展名（小写）
    pub size: u64,       // 文件大小（字节）
    pub mime: String,    // MIME 类型
    pub is_dir: bool,    // 是否为目录
}
```

### DirList

```rust
pub struct DirList {
    pub path: String,          // 当前目录路径
    pub entries: Vec<FileInfo>, // 子项列表
}
```

## 安全特性

### 路径遍历防护

自动拦截包含 `..` 或 `\0` 的路径，`canonicalize` 后校验是否在 root 目录内：

```rust
// ✅ 正常路径
fs.upload("docs/readme.txt", data)?;

// ❌ 路径遍历攻击 → 返回 41905 错误
fs.upload("../../etc/passwd", data)?;
```

### 文件类型校验

配置 `allowed` 列表后，只有列出的扩展名允许上传：

```rust
// 配置 allowed = ["jpg", "png", "pdf"]

// ✅ 允许
fs.upload("photo.jpg", data)?;

// ❌ 拒绝 → 返回 41903 错误
fs.upload("virus.exe", data)?;
```

### 文件大小限制

上传时自动检查文件大小：

```rust
// 配置 max_size = 10485760 (10MB)

// ❌ 超过限制 → 返回 41902 错误
fs.upload("big.zip", &huge_data)?;
```

## 使用示例

```rust
use afaster::{FileConfig, FileService};

let config = FileConfig {
    root: "./uploads".to_string(),
    max_size: 10 * 1024 * 1024,
    allowed: vec!["jpg".into(), "png".into(), "pdf".into()],
    url_prefix: "/files".to_string(),
};

let fs = FileService::new(&config)?;

// 上传
let info = fs.upload("avatars/1.jpg", &image_bytes)?;
println!("已上传: {} ({} bytes)", info.path, info.size);

// 下载
let data = fs.download("avatars/1.jpg")?;

// 生成 URL
let url = fs.url("avatars/1.jpg")?; // "/files/avatars/1.jpg"

// 目录列表
let list = fs.list("avatars")?;
for entry in &list.entries {
    println!("{}: {} bytes", entry.name, entry.size);
}

// 删除
fs.delete("avatars/1.jpg")?;
```

## 支持的 MIME 类型

| 扩展名 | MIME |
|--------|------|
| jpg/jpeg | image/jpeg |
| png | image/png |
| gif | image/gif |
| webp | image/webp |
| svg | image/svg+xml |
| pdf | application/pdf |
| doc/docx | MS Word |
| xls/xlsx | MS Excel |
| zip/rar/7z | 压缩包 |
| txt/csv/md | 文本 |
| mp3/wav/ogg | 音频 |
| mp4/avi/mov | 视频 |
| ttf/otf/woff | 字体 |
| 其他 | application/octet-stream |

## 错误码

| code | 含义 |
|------|------|
| 41901 | 文件不存在 |
| 41902 | 文件过大 |
| 41903 | 文件类型不允许 |
| 41904 | 路径不合法 |
| 41905 | 路径遍历攻击 |
| 51901 | 文件读取失败 |
| 51902 | 文件写入失败 |
| 51903 | 文件删除失败 |
| 51904 | 目录创建失败 |
| 51905 | 目录读取失败 |
