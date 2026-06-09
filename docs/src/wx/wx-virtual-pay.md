# 微信虚拟支付

Feature: `wx-virtual-pay`

> 📖 官方文档：<https://developers.weixin.qq.com/miniprogram/dev/platform-capabilities/business-capabilities/virtual-payment.html>

## 配置

```toml
[wx_virtual_pay]
offer_id = ""                  # 必填, 商户号
app_key = ""                   # 必填, 支付 AppKey
env = 0                        # 0=正式环境, 1=沙箱环境
mini_id = ""                   # 必填, 支付小程序 AppID（独立于登录小程序）
mini_secret = ""               # 必填, 支付小程序 AppSecret
```

## 模块结构

```
src/state/wx_virtual_pay/
├── mod.rs       # 配置、签名、API 请求体构建、独立登录
├── callback.rs  # 推送请求/响应类型定义
├── err.rs       # 错误码
└── session.rs   # WxSessionManager session_key 缓存管理
```

---

## 一、独立登录与 session_key 管理

`wx_virtual_pay` 使用独立的 `mini_id`/`mini_secret` 调用微信 `jscode2Session` 接口，
与 `wx-login-mini` 完全解耦，支持不同小程序。

### WxSessionManager

按 `appid:openid` 为 key 存储 `session_key`，支持多个小程序共存。
内部使用 `Arc<RwLock<HashMap>>` 实现线程安全的跨请求共享。

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `set` | `key: &str, session_key: &str` | - | 存入 |
| `get` | `key: &str` | `Option<String>` | 取出 |
| `remove` | `key: &str` | - | 删除 |
| `contains` | `key: &str` | `bool` | 是否存在 |
| `updated_at` | `key: &str` | `Option<Instant>` | 存入时间 |
| `clear` | - | - | 清除所有 |

### 登录 API

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `mini_login` | `code: &str` | `Result<WxMiniLoginResponse>` | 登录并自动缓存 session_key |
| `cache_session` | `openid: &str, session_key: &str` | - | 手动存入 session_key |
| `get_session` | `openid: &str` | `Option<String>` | 获取缓存的 session_key |
| `session_key_for` | `openid: &str` | `String` | 生成缓存 key `"appid:openid"` |

### WxMiniLoginResponse

```rust
pub struct WxMiniLoginResponse {
    pub openid: String,
    pub session_key: String,
    pub unionid: Option<String>,
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
}
```

### 使用示例

```rust
// 登录（用支付小程序自己的凭证）
let result = state.wx_virtual_pay.mini_login(&code).await?;
// 自动缓存 session_key，key = "appid:openid"

// 获取缓存的 session_key
let session_key = state.wx_virtual_pay.get_session(&result.openid);

// 手动存入已有的 session_key
state.wx_virtual_pay.cache_session(&openid, &session_key);
```

---

## 二、两种购买模式

### 1. 代币充值（Coin）

用户用现金购买代币，代币可用于后续支付。

### 2. 道具直购（Goods）

用户直接用现金购买道具/商品。

---

## 三、签名 API

### 客户端签名

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `coin_sign_data` | `order_no, quantity, attach` | `String` | 代币充值 signData JSON |
| `goods_sign_data` | `order_no, product_id, quantity, goods_price, attach` | `String` | 道具直购 signData JSON |
| `client_pay_sig` | `sign_data_json` | `String` | 客户端 paySig |
| `client_signature` | `session_key, sign_data_json` | `String` | 客户端 signature |

### 服务器端签名

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `calc_pay_sig` | `uri, post_body` | `String` | 服务器端 paySig |
| `calc_signature` | `session_key, post_body` | `String` | 服务器端 signature |

### 签名算法

```
# 客户端 (wx.requestVirtualPayment)
paySig = hmac_sha256(appKey, "requestVirtualPayment&" + signData)
signature = hmac_sha256(sessionKey, signData)

# 服务器端 API
paySig = hmac_sha256(appKey, uri + "&" + post_body)
signature = hmac_sha256(sessionKey, post_body)
```

---

## 四、服务器端 API 请求体构建

### 代币相关

| 方法 | 参数 | 说明 |
|------|------|------|
| `query_user_balance_body` | `openid, user_ip, env` | 查询代币余额 |
| `currency_pay_body` | `openid, user_ip, env, out_trade_no, quantity, attach` | 扣减代币 |
| `cancel_currency_pay_body` | `openid, user_ip, env, out_trade_no` | 代币支付退款 |
| `present_currency_body` | `openid, user_ip, env, out_trade_no, quantity, attach` | 代币赠送 |

### 订单与账单

| 方法 | 参数 | 说明 |
|------|------|------|
| `query_order_body` | `openid, user_ip, env, out_trade_no` | 查询订单 |
| `notify_provide_goods_body` | `openid, user_ip, env, out_trade_no` | 通知已发货完成 |
| `refund_order_body` | `openid, user_ip, env, out_trade_no, refund_out_trade_no, refund_fee` | 启动退款任务 |

### 消息推送签名验证

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `verify_push_signature` | `event, payload, sig` | `bool` | 验证推送签名 |

---

## 五、消息推送回调

> **统一通知模块**：虚拟支付的 4 种推送事件由 `wx-notify` 模块统一接收和分发。
> 微信后台只需配置一个回调 URL（默认 `https://your-domain/wx/notify`），
> 后端通过 `Event` 字段自动路由到对应回调。
>
> 回调通过 `AFaster::new().with_wx_notify(|notify| notify.with_xxx_callback(...))` 注册，
> 详细配置参见 `config.toml` 中的 `[wx_notify]` 段。

### 推送事件

| 事件 | Event 字段 | 说明 |
|------|-----------|------|
| 道具发货 | `xpay_goods_deliver_notify` | 用户现金购买道具支付成功 |
| 代币支付 | `xpay_coin_pay_notify` | 代币扣减成功 |
| 退款 | `xpay_refund_notify` | 退款完成 |
| 用户投诉 | `xpay_complaint_notify` | 用户发起投诉 |

### 回调请求类型

每种推送都有对应的请求体结构体，字段使用 PascalCase 匹配微信推送格式：

| 结构体 | 对应事件 | 关键字段 |
|--------|----------|----------|
| `WxGoodsDeliverNotify` | 道具发货 | `OpenId`, `OutTradeNo`, `Env`, `WeChatPayInfo`, `GoodsInfo`, `TeamInfo` |
| `WxCoinPayNotify` | 代币支付 | `OpenId`, `OutTradeNo`, `Env`, `WeChatPayInfo`, `CoinInfo` |
| `WxRefundNotify` | 退款 | `OpenId`, `WxRefundId`, `RefundFee`, `RetCode`, `RetMsg` |
| `WxComplaintNotify` | 用户投诉 | `OpenId`, `ComplaintId`, `ComplaintDetail`, `TransactionId` |

### 公共嵌套类型

| 结构体 | 说明 |
|--------|------|
| `WeChatPayInfo` | 微信支付信息（`MchOrderNo`, `TransactionId`, `PaidTime`） |
| `GoodsInfo` | 道具信息（`ProductId`, `Quantity`, `OrigPrice`, `ActualPrice`, `Attach`） |
| `CoinInfo` | 代币信息（`Quantity`, `OrigPrice`, `ActualPrice`, `Attach`） |
| `TeamInfo` | 拼团信息（`ActivityId`, `TeamId`, `TeamType`, `TeamAction`） |

### 回调响应

所有回调统一返回 `WxPayNotifyResult`，序列化为 JSON `{"ErrCode":0,"ErrMsg":"success"}`。

```rust
WxPayNotifyResult::success()                      // 成功
WxPayNotifyResult::error(code, "失败原因")          // 失败
```

### 注册回调

回调通过 `wx_notify` 模块注册（`wx-virtual-pay` 自动依赖 `wx-notify`）：

```rust
use afaster::AFaster;
use afaster::state::wx_virtual_pay::WxPayNotifyResult;

AFaster::new()
    .with_wx_notify(|notify| {
        notify
            .with_goods_deliver_callback(|(state, notify)| {
                async move {
                    println!("发货: {:?}", notify.out_trade_no);
                    // 处理发货逻辑...
                    Ok(WxPayNotifyResult::success())
                }
            })
            .with_coin_pay_callback(|(state, notify)| {
                async move {
                    println!("代币支付: {:?}", notify.out_trade_no);
                    Ok(WxPayNotifyResult::success())
                }
            })
            .with_refund_callback(|(state, notify)| {
                async move {
                    println!("退款: {:?}", notify.wx_refund_id);
                    Ok(WxPayNotifyResult::success())
                }
            })
            .with_complaint_callback(|(state, notify)| {
                async move {
                    println!("投诉: {:?}", notify.complaint_id);
                    Ok(WxPayNotifyResult::success())
                }
            })
    })
    .run()
    .await;
```

> **注意**：未注册的回调会被静默丢弃并返回成功（避免微信重试），开启 `log` feature 会打印 debug 日志。建议注册所有 4 种回调。

---

## 六、服务器端 API 列表

| API | 说明 |
|-----|------|
| `/xpay/query_user_balance` | 查询代币余额 |
| `/xpay/currency_pay` | 扣减代币 |
| `/xpay/cancel_currency_pay` | 代币支付退款 |
| `/xpay/present_currency` | 代币赠送 |
| `/xpay/query_order` | 查询订单 |
| `/xpay/notify_provide_goods` | 通知发货完成 |
| `/xpay/refund_order` | 启动退款 |

---

## 七、使用示例

```rust
// ═══ 登录获取 session_key ═══
let result = state.wx_virtual_pay.mini_login(&code).await?;
let session_key = state.wx_virtual_pay.get_session(&result.openid).unwrap();

// ═══ 代币充值 ═══
let sign_data = state.wx_virtual_pay.coin_sign_data("ORDER001", 100, "extra");
let pay_sig = state.wx_virtual_pay.client_pay_sig(&sign_data);
let signature = WxVirtualPay::client_signature(&session_key, &sign_data);

// ═══ 道具直购 ═══
let sign_data = state.wx_virtual_pay.goods_sign_data("ORDER002", "goods_001", 1, 100, "extra");

// ═══ 服务器端 API 调用 ═══
let body = WxVirtualPay::query_user_balance_body(&openid, &ip, 0);
let pay_sig = state.wx_virtual_pay.calc_pay_sig("/xpay/query_user_balance", &body);

// ═══ 验证消息推送 ═══
let valid = state.wx_virtual_pay.verify_push_signature("xpay_goods_deliver_notify", &payload, &sig);
```

---

## 八、错误码

### wx_virtual_pay 模块错误

| 码 | 说明 |
|----|------|
| 40601 | 微信 API 返回登录错误 |
| 40602 | session_key 已过期 |
| 50601 | 登录 URL 解析错误 |
| 50602 | 登录 HTTP 请求失败 |
| 50603 | 登录响应解析失败 |
| 50604 | 登录 JSON 转换失败 |
