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

调用前应在云托管控制台开启开放接口服务、配置 `/tcb/uploadfile` 和 `/tcb/batchdownloadfile` 权限并重新创建服务版本。模块通过 `http://api.weixin.qq.com` 调用，不接触云托管访问令牌；如响应包含 `x-openapi-seqid`，可用它确认请求经过开放接口服务。

本地开发时可使用微信云托管 VS Code 插件模拟开放接口服务；普通本地进程没有该代理，接口调用会失败。

参考：

- [微信小程序对象存储文档](https://developers.weixin.qq.com/miniprogram/dev/wxcloudservice/wxcloudrun/src/development/storage/miniapp/)
- [开放接口服务文档](https://developers.weixin.qq.com/miniprogram/dev/wxcloudservice/wxcloudrun/src/guide/weixin/open)
