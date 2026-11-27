# 微信云托管云调用

Feature: `wx-cloudrun`

云托管开放接口服务通过旁加载代理微信 API，并负责鉴权。服务端请求 `api.weixin.qq.com` 时不需要自行读取、存储或传递 `access_token` / `cloudbase_access_token`。

```toml
afaster = { version = "0.0.0", default-features = false, features = ["wx-cloudrun"] }
```

Configure the CloudBase environment once in `config.toml`. The prefix is prepended to upload paths; `url_expire` sets temporary download URL validity in seconds (1–604800, default 3600).

```toml
[wxcloudrun]
env = "your-cloudbase-env-id"
prefix = "items"
url_expire = 3600
```

## 获取对象上传链接

`tcb/uploadfile` 返回短期上传授权信息。通过开放接口服务调用时，使用 HTTP 且不附带任何 token：

```rust,ignore
let link = state
    .wxcloudrun
    .get_upload_file_link("user-id/image.jpg")
    .await?;
```

请求会发送配置中的 `env` 和带 `prefix` 的路径。响应包含 `url`、`token`、`authorization`、`file_id` 和 `cos_file_id`。其中临时上传授权应只返回给当前已鉴权用户。服务端应生成对象路径并将上传任务关联到用户，不能直接采用客户端任意提交的 COS URL。

## 获取临时访问链接

使用上传成功后保存的 `file_id` 获取短期下载链接：

```rust,ignore
let temporary_url = state
    .wxcloudrun
    .get_temporary_access_url(&link.file_id)
    .await?;
```

方法只接受当前配置环境及 `prefix` 下的 CloudID，并返回链接有效期和可选的 `x-openapi-seqid`。临时链接在有效期内可访问，应按用户权限校验后再发放，不要将它作为永久图片地址存储。

## 临时链接上的 COS 图片处理

`get_temporary_access_url_with_image_process` 会先通过 CloudBase 获取临时下载链接，再将 COS 数据万象图片处理指令追加到该链接的查询串。处理在下载时执行，不会修改或覆盖原对象：

```rust,ignore
let thumbnail = state
    .wxcloudrun
    .get_temporary_access_url_with_image_process(
        &link.file_id,
        "imageMogr2/thumbnail/!50p/quality/85/format/webp",
    )
    .await?;
```

`process` 是 COS CI URL 指令片段，不要带开头的 `?` 或 `&`。例如可使用 `imageMogr2` 的缩放、裁剪、旋转、质量和格式转换指令，以及 `watermark` 水印指令；其他 COS CI URL 指令也可按服务端支持情况传入。若参数包含 `&`、`?`、`#`、空格或 `+` 等保留字符，应按 COS 规则先进行百分号编码；原始 `&`、`?`、`#`、空格、反斜线、控制字符和无效百分号编码会被拒绝，避免改变临时 URL 的授权参数。

图片处理需要在对应 COS 存储桶开通并配置数据万象。CloudBase 临时 URL 带有短期授权，COS 是否接受附加的处理指令取决于该 URL 和存储桶配置；应先用测试图片验证 HTTP 状态码、返回格式与签名有效性。若授权签名不允许附加处理参数，可改为由 COS CI 生成并保存派生图片，再对派生对象申请临时访问链接。该接口只处理下载响应，不支持上传时 `Pic-Operations` 持久化处理、异步任务或视频/文档处理流程。

调用前应在云托管控制台开启开放接口服务、配置 `/tcb/uploadfile` 和 `/tcb/batchdownloadfile` 权限并重新创建服务版本。模块通过 `http://api.weixin.qq.com` 调用，不接触云托管访问令牌；如响应包含 `x-openapi-seqid`，可用它确认请求经过开放接口服务。

本地开发时可使用微信云托管 VS Code 插件模拟开放接口服务；普通本地进程没有该代理，接口调用会失败。

仓库提供 `wxcloudrun_ci_image_smoke` 示例，可在开放接口代理可用的环境中从 `item_images` 读取最近一张图片，分别下载原图和 50% 缩略、质量 80、WebP 格式的响应，并仅输出 HTTP 状态、MIME 类型和字节数：

```powershell
cargo run --example wxcloudrun_ci_image_smoke --features wx-cloudrun,orm-mysql -- <backend-config-path>
```

参考：

- [微信小程序对象存储文档](https://developers.weixin.qq.com/miniprogram/dev/wxcloudservice/wxcloudrun/src/development/storage/miniapp/)
- [开放接口服务文档](https://developers.weixin.qq.com/miniprogram/dev/wxcloudservice/wxcloudrun/src/guide/weixin/open)
