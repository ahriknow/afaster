# Snowflake ID 生成器

Feature: `snow`

## 配置

```toml
[snow]
epoch = 0              # 自定义起始时间（毫秒时间戳），0 = 使用当前时间
worker_id = 1          # 机器 ID (0~31)
datacenter_id = 1      # 数据中心 ID (0~31)
```

## API

### SnowConfig

配置结构体，从 `config.toml` 读取。

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `epoch` | `i64` | `0` | 起始时间戳（毫秒），0 表示当前时间 |
| `worker_id` | `i64` | `1` | 机器 ID (0~31) |
| `datacenter_id` | `i64` | `1` | 数据中心 ID (0~31) |

### Snowflake

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `from_config` | `config: &SnowConfig` | `Snowflake` | 从配置创建实例 |
| `next_id` | - | `i64` | 生成下一个唯一 ID |
| `next_id_str` | - | `String` | 生成下一个唯一 ID（字符串） |
| `next_id_prefix` | `prefix: &str` | `String` | 生成带前缀的唯一 ID |

## 使用示例

```rust
// 通过 AppState 访问（已从 config 自动初始化）
let id = state.snow.next_id();
let id_str = state.snow.next_id_str();
let order_no = state.snow.next_id_prefix("ORD");
```

## ID 结构

```
| 1 bit | 41 bits timestamp | 5 bits datacenter | 5 bits worker | 12 bits sequence |
```

- 时间精度: 毫秒
- 理论最大: ~69 年
- 单机每毫秒: 4096 个 ID
