# Excel / CSV

Feature: `excel` | 依赖: `calamine`, `rust_xlsxwriter`, `csv`

## 简介

表格文件读写工具。支持读取 `.xlsx` / `.xls` / `.xlsb` / `.ods` / `.csv`，写入 `.xlsx` / `.csv`。

无需实例化，`Excel` 为静态工具结构体，所有方法均可直接调用。

**自动识别**：`read_path` 根据文件扩展名自动选择解析器（`.csv` → CSV，其他 → Excel）。

**单 Sheet**：当前所有 API 操作单个工作表。Excel 可通过 `sheet` 参数选择 sheet，CSV 无 sheet 概念。

**大文件**：当前 `Excel` API 全量加载到内存。超大文件请使用 `RowReader` / `RowWriter` 流式 API。

## 依赖配置

```toml
[dependencies]
afaster = { version = "0.0.7", features = ["excel"] }
```

## 读取 API

### read_path — 自动识别格式读取

根据文件扩展名自动选择解析器：

```rust
use afaster::Excel;

// .csv → CSV 解析器；.xlsx → Excel 解析器
let data = Excel::read_path("data.csv", None)?;
let data = Excel::read_path("data.xlsx", None)?;
let data = Excel::read_path("data.xlsx", Some("Sheet1"))?; // Excel 指定 sheet
// 返回 Vec<Vec<CellValue>>
```

### read_bytes — 从字节数据读取

适合 HTTP 上传、数据库 BLOB 等场景：

```rust
let bytes = reqwest::get(url).await?.bytes().await?;
let data = Excel::read_bytes(&bytes, None)?;
```

### read_bytes_as — 指定或自动检测格式

```rust
use afaster::Format;

let data = Excel::read_bytes_as(&bytes, Some(Format::Csv), None)?;    // 强制 CSV
let data = Excel::read_bytes_as(&bytes, Some(Format::Xlsx), None)?;   // 强制 Excel
let data = Excel::read_bytes_as(&bytes, None, None)?;                  // 自动检测
```

### read_all — 读取所有工作表

```rust
let sheets = Excel::read_all("multi.xlsx")?;
// CSV 返回单个 sheet，key 为 "Sheet1"
let sheets = Excel::read_all("data.csv")?;
for (name, rows) in &sheets {
    println!("📄 {}: {} 行", name, rows.len());
}
```

### read_skip — 跳过前 N 行

```rust
let data = Excel::read_skip("data.xlsx", None, 2)?; // 跳过前 2 行
```

### read_with_headers — 第一行作为表头

```rust
let (headers, rows) = Excel::read_with_headers("data.csv", None)?;
println!("表头: {:?}", headers);
```

### read_to_json — 读取为 JSON 数组

```rust
let json = Excel::read_to_json("users.xlsx", None)?;
for item in &json {
    println!("{}", serde_json::to_string(item)?);
}
```

### 显式格式读取

```rust
// 显式 CSV
let data = Excel::read_csv_path("data.csv")?;
let data = Excel::read_csv_bytes(&csv_bytes)?;

// 显式 Excel
let data = Excel::read_xlsx_path("data.xlsx", Some("Sheet1"))?;
let data = Excel::read_xlsx_bytes(&xlsx_bytes, None)?;
let sheets = Excel::read_xlsx_all("multi.xlsx")?;
```

## 写入 API

### write / write_as — 纯字符串写入

```rust
use afaster::{Excel, Format};

// 默认 XLSX
let bytes = Excel::write(&[&["Name", "Age"], &["Alice", "25"]])?;

// 指定格式
let bytes = Excel::write_as(&[&["Name", "Age"], &["Alice", "25"]], Format::Csv)?;
let bytes = Excel::write_as(&[&["Name", "Age"], &["Alice", "25"]], Format::Xlsx)?;
```

### write_typed / write_typed_as — 带类型写入

```rust
use afaster::{Excel, CellValue, Format};

let data = vec![
    vec![CellValue::String("Name".into()), CellValue::String("Age".into())],
    vec![CellValue::String("Alice".into()), CellValue::Int(25)],
];

// XLSX（保留类型）
let bytes = Excel::write_typed(&data, Some("Users"))?;
// CSV（数字/布尔自动转字符串）
let bytes = Excel::write_typed_as(&data, Format::Csv, None)?;
```

### write_with_headers / write_with_headers_as — 带表头写入

```rust
let headers = vec!["Language", "Rating"];
let rows = vec![
    vec![CellValue::String("Rust".into()), CellValue::Int(10)],
];

let bytes = Excel::write_with_headers(&headers, &rows, Some("Sheet1"))?;
let bytes = Excel::write_with_headers_as(&headers, &rows, Format::Csv, None)?;
```

### write_multi / write_multi_as — 多工作表写入

```rust
let sheets = vec![
    ("Users", vec![...]),
    ("Orders", vec![...]),
];

let bytes = Excel::write_multi(&sheets)?;                       // XLSX 多 sheet
let bytes = Excel::write_multi_as(&sheets, Format::Csv)?;       // CSV 仅写第一个 sheet
```

## Format 枚举

```rust
use afaster::Format;

Format::Xlsx  // Excel .xlsx 格式
Format::Csv   // CSV 格式
```

## CSV 特性

- **智能类型推断**：读取 CSV 时自动将数字解析为 `Int`/`Float`，`true`/`false`/`yes`/`no`/`1`/`0` 解析为 `Bool`
- **单 Sheet**：CSV 无 sheet 概念，`read_all` 返回单个 `"Sheet1"`
- **多 Sheet 写入降级**：`write_multi_as` 对 CSV 仅写第一个 sheet
- **无格式信息**：CSV 中所有类型写入时转为字符串，读取时重新推断

## CellValue 类型

| 变体 | 说明 | Excel | CSV 读取时 |
|------|------|-------|-----------|
| `Empty` | 空值 | 空单元格 | 空字段 |
| `String(String)` | 字符串 | 文本 | 无法解析为数字/布尔的文本 |
| `Float(f64)` | 浮点数 | 数字（有小数） | 含小数点的数字 |
| `Int(i64)` | 整数 | 数字（无小数） | 整数 |
| `Bool(bool)` | 布尔值 | TRUE / FALSE | true/false/yes/no/1/0 |
| `Error(String)` | 错误 | #ERR | — |

### CellValue 方法

```rust
let cell = CellValue::Int(42);

cell.as_str()    // Option<&str> — 仅 String 变体返回 Some
cell.as_f64()    // Option<f64> — Float/Int 返回 Some
cell.as_i64()    // Option<i64> — Int/Float 返回 Some
cell.as_bool()   // Option<bool> — 仅 Bool 变体返回 Some
cell.is_empty()  // bool
cell.to_json()   // serde_json::Value
cell.to_string() // String（Display trait）
```

## 完整示例

```rust
use afaster::{Excel, CellValue, Format};

// 自动识别读取 CSV
let (headers, rows) = Excel::read_with_headers("users.csv", None)?;

// 导出为 CSV
let bytes = Excel::write_with_headers_as(
    &headers, &rows, Format::Csv, None
)?;
std::fs::write("export.csv", bytes)?;

// 导出为 Excel
let bytes = Excel::write_with_headers_as(
    &headers, &rows, Format::Xlsx, Some("Users")
)?;
std::fs::write("export.xlsx", bytes)?;
```

## 流式 API

大文件场景使用 `RowReader`（逐行读取）和 `RowWriter`（逐行写入），避免一次性加载全部数据到内存。

### RowReader — 流式读取

CSV 真正逐行流式读取；Excel 内部加载后逐行迭代（calamine 限制）。

```rust
use afaster::RowReader;

// 自动识别格式
let mut reader = RowReader::from_path("large.csv", None)?;

// 提取表头（跳过第一行）
let headers = reader.headers(); // Option<Vec<String>>

// 逐行读取
while let Some(row) = reader.next_row() {
    let row = row?; // Vec<CellValue>
    println!("{:?}", row);
}

// 或使用 Iterator trait
let reader = RowReader::from_path("data.xlsx", Some("Sheet1"))?;
for row in reader {
    let cells = row?;
    // ...
}

// 收集剩余行
let mut reader = RowReader::from_csv_path("data.csv")?;
reader.headers(); // 跳过表头
let data_rows = reader.collect_remaining()?;
```

#### RowReader 构造方法

| 方法 | 说明 |
|------|------|
| `from_path(path, sheet)` | 自动识别格式 |
| `from_csv_path(path)` | CSV 文件流式读取 |
| `from_csv_bytes(bytes)` | CSV 字节流式读取 |
| `from_xlsx_path(path, sheet)` | Excel 文件 |
| `from_xlsx_bytes(bytes, sheet)` | Excel 字节 |

#### RowReader 方法

| 方法 | 说明 |
|------|------|
| `headers()` | 提取第一行为表头 |
| `next_row()` | 读取下一行 `Option<Result<Vec<CellValue>>>` |
| `total_rows()` | 总行数（仅 Excel） |
| `sheet_names()` | 工作表名列表（仅 Excel） |
| `selected_sheet()` | 当前工作表名（仅 Excel） |
| `collect_remaining()` | 收集剩余所有行 |

支持 `Iterator` trait，可直接用于 `for` 循环。

### RowWriter — 流式写入

逐行写入，最后一次性生成输出。

```rust
use afaster::{RowWriter, CellValue, Format};

// 方式 1：内存积累，最后生成字节
let mut writer = RowWriter::new();
writer.write_row_str(&["Name", "Age"])?;
writer.write_row(&[CellValue::String("Alice".into()), CellValue::Int(25)])?;
writer.write_row(&[CellValue::String("Bob".into()), CellValue::Int(30)])?;

let csv_bytes = writer.finish_csv_bytes()?;
let xlsx_bytes = writer.finish_xlsx_bytes(Some("Sheet1"))?;
let bytes = writer.finish_bytes(Format::Csv)?; // 指定格式

// 方式 2：直接写入 CSV 文件（边写边输出，不积累内存）
let mut writer = RowWriter::from_csv_path("output.csv")?;
writer.write_row_str(&["Name", "Age"])?;
writer.write_row(&[CellValue::String("Alice".into()), CellValue::Int(25)])?;
writer.finish()?; // 关闭文件

// 方式 3：保存到文件
let mut writer = RowWriter::new();
writer.write_row_str(&["Name", "Age"])?;
writer.write_row(&[CellValue::String("Alice".into()), CellValue::Int(25)])?;
writer.finish_csv_path("output.csv")?;
writer.finish_xlsx_path("output.xlsx", None)?;
writer.finish_path("output.csv", Format::Csv)?; // 指定格式
```

#### RowWriter 构造方法

| 方法 | 说明 |
|------|------|
| `new()` | 内存积累模式 |
| `from_csv_path(path)` | 直接写入 CSV 文件（流式输出） |

#### RowWriter 方法

| 方法 | 说明 |
|------|------|
| `write_row(&[CellValue])` | 写入一行 |
| `write_row_str(&[&str])` | 写入一行纯字符串 |
| `len()` / `is_empty()` | 已写入行数 |
| `finish()` | 关闭文件（仅 `from_csv_path` 模式） |
| `finish_csv_bytes()` | 生成 CSV 字节 |
| `finish_xlsx_bytes(sheet)` | 生成 Excel 字节 |
| `finish_bytes(format)` | 指定格式生成字节 |
| `finish_csv_path(path)` | 保存 CSV 文件 |
| `finish_xlsx_path(path, sheet)` | 保存 Excel 文件 |
| `finish_path(path, format)` | 指定格式保存文件 |

## 错误码

| code | 含义 |
|------|------|
| 51601 | 打开 Excel 文件失败（文件不存在、格式错误等） |
| 51602 | 工作表不存在或读取失败 |
| 51603 | 写入 Excel 失败 |
| 51604 | CSV 解析失败 |
