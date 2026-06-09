# JWT Token 认证

Feature: `jwt`

## 配置

```toml
[token]
expire = 3600    # 过期时间（小时）
secret = "your-secret-key"
```

## API

### Token

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `create_token<T>` | `user_id: i64, data: T` | `Result<String>` | 生成 JWT，嵌入泛型数据 |
| `verify_token<T>` | `token: &str` | `Result<Claims<T>>` | 验证并解析 JWT（含泛型数据） |
| `get_id` | `token: &str` | `Result<i64>` | 从 JWT 提取 user_id |
| `get_data<T>` | `token: &str` | `Result<T>` | 验证 JWT 并提取嵌入的泛型数据 |

### Claims

```rust
pub struct Claims<T = ()> {
    pub sub: String,      // user_id 字符串
    pub exp: usize,       // 过期时间戳
    pub uid: i64,         // user_id
    pub data: Option<T>,  // 嵌入的泛型数据
}
```

> `T` 默认为 `()`，此时 `data` 为 `None`。调用 `get_id` 时使用 `Claims<()>` 解析，忽略 data 字段。

## 错误码

| 码 | English | 中文 |
|----|---------|------|
| 40101 | Invalid token | 无效的令牌 |
| 40102 | Token expired | 令牌已过期 |
| 50101 | JWT encode failed | JWT 编码失败 |
| 50102 | JWT verify failed | JWT 验证失败 |
| 50103 | Token data not found | 令牌中未找到嵌入数据 |

## 使用示例

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct UserData {
    id: i64,
    username: String,
    is_super: bool,
}

// 生成 Token（嵌入自定义数据）
let data = UserData { id: 1, username: "admin".into(), is_super: true };
let token = state.token.create_token(user_id, &data)?;

// 提取 user_id（忽略 data）
let user_id = state.token.get_id(&token)?;

// 验证并获取完整声明
let claims = state.token.verify_token::<UserData>(&token)?;
println!("user: {}, data: {:?}", claims.uid, claims.data);

// 直接获取嵌入的数据
let data = state.token.get_data::<UserData>(&token)?;
println!("username: {}", data.username);
```
