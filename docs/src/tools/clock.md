# 时钟工具

Feature: `clock`

## API

### Clock

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `new` | - | `Clock` | 创建实例 |
| `now_s` | - | `i64` | 当前时间戳（秒） |
| `now_ms` | - | `i64` | 当前时间戳（毫秒） |

## 使用示例

```rust
let timestamp_sec = state.clock.now_s();
let timestamp_ms = state.clock.now_ms();
```
