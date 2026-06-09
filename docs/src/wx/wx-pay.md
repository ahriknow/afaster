# 微信支付 V3 (wx-pay)

Feature: `wx-pay-h5` / `wx-pay-native` / `wx-pay-app` / `wx-pay-mini` / `wx-pay-js`

> 📖 官方文档：<https://pay.weixin.qq.com/doc/v3/merchant/4012791832\>（产品介绍）

基于微信支付 V3 API，使用 RSA-SHA256 签名，统一支持五种支付方式。

## 产品概述

| 支付方式 | Feature | 场景 | 预下单返回 | 调起方式 |
|---------|---------|------|-----------|---------|
| H5 支付 | `wx-pay-h5` | 手机浏览器网页 | `h5_url` | 前端重定向到 h5_url |
| Native 支付 | `wx-pay-native` | PC 端扫码 | `code_url` | 将 code_url 转换为二维码 |
| APP 支付 | `wx-pay-app` | 商户 APP 内 | `prepay_id` | APP 端调起微信 SDK |
| 小程序支付 | `wx-pay-mini` | 微信小程序内 | `prepay_id` | `wx.requestPayment` |
| JSAPI 支付 | `wx-pay-js` | 微信内置浏览器 | `prepay_id` | `WeixinJSBridge.invoke` |

> 📖 [H5 产品介绍](wx-pay/H5产品介绍.md) | [Native 产品介绍](wx-pay/Native产品介绍.md) | [APP 产品介绍](wx-pay/APP产品介绍.md) | [小程序/JSAPI 产品介绍](wx-pay/小程序产品介绍.md)

## 配置

三种支付方式共用一个 `[wx_pay]` 配置段：

```toml
[wx_pay]
mch_id = ""                          # 商户号
app_id = ""                          # 应用 ID (AppID)
api_v3_key = ""                      # API v3 密钥 (32 字节)
private_key = """                    # 商户 API 私钥 (PEM 格式)
-----BEGIN PRIVATE KEY...
...
-----END PRIVATE KEY...
"""
serial_no = ""                       # 商户证书序列号
notify_url = ""                      # 支付回调完整 URL (发给微信)
# callback_path = "wx/pay/notify"    # 回调路由路径, 默认 wx/pay/notify
# refund_notify_url = ""             # 退款回调完整 URL (可选, 默认使用 notify_url)
# platform_cert = """                # 微信支付平台证书公钥 (PEM 格式, 用于验签回调)
# """
```

## 使用方法

### 1. 预下单

> 📖 [H5下单](wx-pay/H5下单.md) | [Native下单](wx-pay/Native下单.md) | [APP下单](wx-pay/APP下单.md)

**H5 支付**（`wx-pay-h5`）：

```rust
let result = state.wx_pay.prepay_h5(
    "ORDER_20260603_001",     // 商户订单号
    "测试商品",                // 商品描述
    100,                      // 金额 (分)
    "123.123.123.123",        // 用户终端 IP
    "Wap",                    // 场景类型 (Wap/IOS/Android)
    "https://example.com",    // 场景 URL
).await?;
println!("h5_url: {}", result.h5_url);  // 前端重定向到此 URL
```

**Native 支付**（`wx-pay-native`）：

```rust
let result = state.wx_pay.prepay_native(
    "ORDER_20260603_001",     // 商户订单号
    "测试商品",                // 商品描述
    100,                      // 金额 (分)
).await?;
println!("code_url: {}", result.code_url);  // 转换为二维码展示
```

**APP 支付**（`wx-pay-app`）：

```rust
let result = state.wx_pay.prepay_app(
    "ORDER_20260603_001",     // 商户订单号
    "测试商品",                // 商品描述
    100,                      // 金额 (分)
).await?;
println!("prepay_id: {}", result.prepay_id);  // APP 端调起微信 SDK
```

**小程序支付**（`wx-pay-mini`）：

```rust
let result = state.wx_pay.prepay_mini(
    "ORDER_20260603_001",     // 商户订单号
    "测试商品",                // 商品描述
    100,                      // 金额 (分)
    "oUpF8uMuAJO_M2pxb1Q9zNjWeS6o", // 用户 openid
).await?;
println!("prepay_id: {}", result.prepay_id);  // wx.requestPayment 调起支付
```

**JSAPI 支付**（`wx-pay-js`）：

```rust
let result = state.wx_pay.prepay_js(
    "ORDER_20260603_001",     // 商户订单号
    "测试商品",                // 商品描述
    100,                      // 金额 (分)
    "oUpF8uMuAJO_M2pxb1Q9zNjWeS6o", // 用户 openid
).await?;
println!("prepay_id: {}", result.prepay_id);  // WeixinJSBridge.invoke 调起支付
```

### 2. 查询订单

> 📖 [商户订单号查询](wx-pay/商户订单号查询订单.md) | [微信支付订单号查询](wx-pay/微信支付订单号查询订单.md)

```rust
let order = state.wx_pay.query_by_out_trade_no("ORDER_20260603_001").await?;
println!("交易状态: {}", order.trade_state);

let order = state.wx_pay.query_by_transaction_id("4200001234202606030000000000").await?;
```

### 3. 关闭订单

> 📖 [关闭订单 API 文档](wx-pay/关闭订单.md)

```rust
state.wx_pay.close_order("ORDER_20260603_001").await?;
```

### 4. 申请退款

> 📖 [退款申请 API 文档](wx-pay/退款申请.md)

```rust
let refund = state.wx_pay.refund(
    "REFUND_20260603_001",
    "ORDER_20260603_001",
    100, 100,
    Some("用户申请退款"),
).await?;
```

### 5. 查询退款

> 📖 [查询单笔退款](wx-pay/查询单笔退款（按商户退款单号）.md)

```rust
let refund = state.wx_pay.query_refund("REFUND_20260603_001").await?;
```

### 6. 发起异常退款

> 📖 [发起异常退款](wx-pay/发起异常退款.md)

```rust
let refund = state.wx_pay.apply_abnormal_refund(
    "5000000001202606030000000000",
    "REFUND_20260603_001",
    "USER_BANK_CARD",
).await?;
```

### 7. 申请交易账单

> 📖 [申请交易账单](wx-pay/申请交易账单.md)

```rust
let bill = state.wx_pay.apply_trade_bill("2026-06-02", Some("ALL")).await?;
```

### 8. 申请资金账单

> 📖 [申请资金账单](wx-pay/申请资金账单.md)

```rust
let bill = state.wx_pay.apply_fund_flow_bill("2026-06-02", Some("BASIC")).await?;
```

### 9. 下载账单文件

> 📖 [下载账单](wx-pay/下载账单.md)

```rust
let csv_content = state.wx_pay.download_bill(&bill.download_url).await?;
std::fs::write("bill.csv", &csv_content)?;
```

### 10. 处理回调通知

> 📖 [支付成功回调通知](wx-pay/支付成功回调通知.md) | [退款结果回调通知](wx-pay/退款结果回调通知.md)

`AFaster::run()` 自动注册回调路由，用户只需注册业务回调：

```rust
use afaster::wx_pay::WxPayNotifyResult;

let app = AFaster::new("config.toml".to_string()).await.unwrap()
    .with_wx_pay(|wp| {
        wp.with_pay_success_callback(|state, notify| async move {
            println!("支付成功: {} - {} 分", notify.out_trade_no,
                notify.amount.as_ref().map(|a| a.total).unwrap_or(0));
            Ok(WxPayNotifyResult::success())
        })
        .with_refund_callback(|state, notify| async move {
            println!("退款: {} - {}", notify.out_refund_no, notify.refund_status);
            Ok(WxPayNotifyResult::success())
        })
    });
```

回调处理流程：
1. 微信 POST 到 `notify_url`
2. 框架自动验签（如配置了 `platform_cert`）
3. 按 `event_type` 分发：
   - `TRANSACTION.SUCCESS` → `pay_success_callback`
   - `REFUND.SUCCESS` / `REFUND.ABNORMAL` / `REFUND.CLOSED` → `refund_callback`
4. 未注册回调时静默返回成功，避免微信重试

## 支付流程

> 📖 [H5调起支付](wx-pay/H5调起支付.md) | [Native调起支付](wx-pay/Native调起支付.md) | [APP调起支付](wx-pay/APP调起支付.md)

```
商户 → 调用 prepay_xxx() → 获取 h5_url / code_url / prepay_id
    ↓
用户确认支付（重定向/扫码/SDK 调起）
    ↓
微信回调 notify_url → 后端解密通知 → 处理业务逻辑
```

## 错误码

| 错误码 | 说明 |
|--------|------|
| 40901 | 签名验证失败 |
| 40902 | 微信 API 业务错误 |
| 40903 | 回调通知解密失败 |
| 40904 | 回调通知验签失败 |
| 40905 | 配置缺少必要字段 |
| 50901 | 私钥加载失败 |
| 50902 | 签名生成失败 |
| 50903 | 请求失败 |
| 50904 | 预下单请求失败 |
| 50905 | 预下单响应解析失败 |
| 50907 | 查询订单响应解析失败 |
| 50910 | 退款响应解析失败 |
| 50911 | 支付通知解析失败 |
| 50912 | 退款通知解析失败 |
| 50913 | 证书解析失败 |
| 50914 | 账单响应解析失败 |

## 与虚拟支付的区别

| 维度 | wx-pay | wx-virtual-pay |
|------|--------|----------------|
| 场景 | H5/PC/APP | 微信小程序内 |
| API 版本 | V3 (RSA-SHA256) | V2 (HMAC-SHA256) |
| 签名方式 | 商户私钥 RSA 签名 | appKey HMAC 签名 |
| 回调加密 | AEAD_AES_256_GCM | AES-256-CBC |
| 支付方式 | h5_url / code_url / prepay_id | 小程序 SDK 调起 |
