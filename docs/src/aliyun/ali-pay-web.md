# 支付宝电脑网站支付

> Feature: `ali-pay-web`

支付宝电脑网站支付，商户在电脑网页展示商品或服务，用户确认后跳转支付宝收银台完成付款。纯 Rust 实现，RSA2 签名。

> 📖 官方文档：<https://opendocs.alipay.com/open/270/105899>

## 配置

```toml
[ali_pay]
app_id = "2014072300007148"              # 支付宝应用 ID
private_key = """
-----BEGIN PRIVATE KEY-----
...
-----END PRIVATE KEY-----
"""                                        # 应用私钥 (PKCS#8 PEM 格式)
alipay_public_key = """
-----BEGIN PUBLIC KEY-----
...
-----END PUBLIC KEY-----
"""                                        # 支付宝公钥 (用于验签)
notify_url = "https://example.com/ali/pay/notify"  # 异步通知回调 URL
# return_url = "https://example.com/ali/pay/return" # 同步跳转 URL (可选)
# gateway = "https://openapi.alipay.com/gateway.do" # API 网关 (默认值)
# callback_path = "ali/pay/notify"                   # 回调路由路径 (默认值)
```

## API

### AliPay

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `page_pay` | `&PagePayRequest` | `Result<String>` | 生成支付表单 HTML（自动跳转） |
| `query_trade` | `out_trade_no?, trade_no?` | `Result<AliPayResponse<TradeQueryData>>` | 查询交易状态 |
| `close_trade` | `out_trade_no?, trade_no?` | `Result<AliPayResponse<TradeCloseData>>` | 关闭未付款交易 |
| `refund` | `refund_amount, out_trade_no?, trade_no?, reason?, out_request_no?` | `Result<AliPayResponse<RefundData>>` | 发起退款 |
| `query_refund` | `out_request_no, out_trade_no?, trade_no?` | `Result<AliPayResponse<RefundQueryData>>` | 查询退款状态 |
| `query_bill_download_url` | `bill_type, bill_date` | `Result<AliPayResponse<BillDownloadData>>` | 获取对账单下载地址 |

### PagePayRequest

```rust
pub struct PagePayRequest {
    pub out_trade_no: String,          // 商户订单号 (64 字符以内)
    pub total_amount: String,          // 订单总金额 (元，精确到小数点后两位)
    pub subject: String,               // 订单标题
    pub product_code: Option<String>,  // 产品码，默认 FAST_INSTANT_TRADE_PAY
    pub qr_pay_mode: Option<String>,   // 扫码方式: 0/1/2/3/4
    pub qrcode_width: Option<u64>,     // 二维码宽度 (qr_pay_mode=4)
    pub time_expire: Option<String>,   // 绝对超时时间 yyyy-MM-dd HH:mm:ss
    pub integration_type: Option<String>, // PCWEB / ALIAPP
}
```

### TradeQueryData

| 字段 | 类型 | 说明 |
|------|------|------|
| `trade_no` | `Option<String>` | 支付宝交易号 |
| `out_trade_no` | `Option<String>` | 商户订单号 |
| `trade_status` | `Option<String>` | 交易状态 |
| `total_amount` | `Option<String>` | 订单金额 |
| `buyer_pay_amount` | `Option<String>` | 买家付款金额 |
| `receipt_amount` | `Option<String>` | 实收金额 |

## 交易状态

| 状态 | 说明 |
|------|------|
| `WAIT_BUYER_PAY` | 等待买家付款 |
| `TRADE_CLOSED` | 未付款交易超时关闭 |
| `TRADE_SUCCESS` | 交易支付成功 |
| `TRADE_FINISHED` | 交易结束，不可退款 |

## 使用示例

```rust
use afaster::AFaster;
use afaster::ali_pay::{AliPay, PagePayRequest};

#[tokio::main]
async fn main() {
    AFaster::new("config.toml".to_string()).await
        .unwrap()
        .with_ali_pay(|a| {
            a.with_pay_success_callback(|(state, notify)| {
                async move {
                    println!("支付宝支付成功: {} - {}", notify.out_trade_no, notify.total_amount);
                    Ok(afaster::ali_pay::AliPayNotifyResult::success())
                }
            })
        })
        .run()
        .await;
}
```

### 生成支付页面

```rust
let html = state.ali_pay.page_pay(&PagePayRequest {
    out_trade_no: "ORDER_20240101_001".to_string(),
    total_amount: "88.88".to_string(),
    subject: "测试商品".to_string(),
    product_code: Some("FAST_INSTANT_TRADE_PAY".to_string()),
    qr_pay_mode: None,
    qrcode_width: None,
    time_expire: None,
    integration_type: None,
})?;
// 返回 HTML 表单，浏览器渲染后自动跳转到支付宝收银台
```

### 查询交易

```rust
let result = state.ali_pay.query_trade(Some("ORDER_20240101_001"), None).await?;
if result.code == "10000" {
    if let Some(data) = &result.data {
        println!("交易状态: {:?}", data.trade_status);
    }
}
```

### 退款

```rust
let result = state.ali_pay.refund(
    "10.00",
    Some("ORDER_20240101_001"),
    None,
    Some("用户申请退款"),
    Some("REFUND_001"),
).await?;
```

## 回调系统

开启 `ali-pay-web` feature 后，回调路由自动注册为 POST 端点，无需手动注册 handler。
路由路径由配置中的 `callback_path` 决定，默认 `ali/pay/notify`。

### 注册回调

```rust
AFaster::new("config.toml".to_string()).await
    .unwrap()
    .with_ali_pay(|a| {
        a.with_pay_success_callback(|(state, notify)| {
            async move {
                // notify: AliPayTradeNotify
                // 处理支付成功逻辑
                Ok(afaster::ali_pay::AliPayNotifyResult::success())
            }
        })
        .with_refund_callback(|(state, notify)| {
            async move {
                // notify: AliPayRefundNotify
                // 处理退款通知
                Ok(afaster::ali_pay::AliPayNotifyResult::success())
            }
        })
    })
    .run()
    .await;
```

> **注意**：未注册的回调会被静默丢弃并返回 success（避免支付宝重试），开启 `log` feature 会打印 debug 日志。

## 错误码

| 码 | English | 中文 |
|----|---------|------|
| 42601 | Notify signature verification failed | 回调通知验签失败 |
| 42602 | Missing notify parameters | 回调通知参数缺失 |
| 52601 | Private key load failed | 私钥加载失败 |
| 52602 | Public key load failed | 公钥加载失败 |
| 52603 | Signature failed | 签名失败 |
| 52604 | Signature verification failed | 验签失败 |
| 52605 | Request failed | 请求发送失败 |
| 52606 | Response parse failed | 响应解析失败 |
| 52607 | Callback not registered | 回调函数未注册 |

## 参考文档

- [电脑网站支付产品介绍](https://opendocs.alipay.com/open/270/105899)
- [统一收单下单并支付页面接口](https://opendocs.alipay.com/apis/api_1/alipay.trade.page.pay)
- [统一收单交易查询](https://opendocs.alipay.com/apis/api_1/alipay.trade.query)
- [统一收单交易关闭](https://opendocs.alipay.com/apis/api_1/alipay.trade.close)
- [统一收单交易退款](https://opendocs.alipay.com/apis/api_1/alipay.trade.refund)
- [退款查询](https://opendocs.alipay.com/apis/api_1/alipay.trade.fastpay.refund.query)
- [账单下载](https://opendocs.alipay.com/apis/api_1/alipay.data.dataservice.bill.downloadurl.query)
