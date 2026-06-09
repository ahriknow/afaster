use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor};
use std::path::Path;

use calamine::{Data, Reader, Xlsx, open_workbook};

// ═══════════════════════════════════════════════════════════════
//  Excel / CSV 导入导出
// ═══════════════════════════════════════════════════════════════

/// 写入格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Excel .xlsx 格式
    Xlsx,
    /// CSV 格式
    Csv,
}

/// 单元格值
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    /// 空值
    Empty,
    /// 字符串
    String(String),
    /// 浮点数
    Float(f64),
    /// 整数（从浮点数转换）
    Int(i64),
    /// 布尔值
    Bool(bool),
    /// 错误
    Error(String),
}

impl CellValue {
    /// 转为字符串
    pub fn as_str(&self) -> Option<&str> {
        match self {
            CellValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// 转为 f64
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            CellValue::Float(f) => Some(*f),
            CellValue::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// 转为 i64
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            CellValue::Int(i) => Some(*i),
            CellValue::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    /// 转为 bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            CellValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        matches!(self, CellValue::Empty)
    }

    /// 转为 serde_json::Value
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            CellValue::Empty => serde_json::Value::Null,
            CellValue::String(s) => serde_json::Value::String(s.clone()),
            CellValue::Float(f) => serde_json::json!(f),
            CellValue::Int(i) => serde_json::json!(i),
            CellValue::Bool(b) => serde_json::json!(b),
            CellValue::Error(e) => serde_json::Value::String(format!("#ERR: {}", e)),
        }
    }
}

impl std::fmt::Display for CellValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CellValue::Empty => write!(f, ""),
            CellValue::String(s) => write!(f, "{}", s),
            CellValue::Float(v) => write!(f, "{}", v),
            CellValue::Int(v) => write!(f, "{}", v),
            CellValue::Bool(v) => write!(f, "{}", v),
            CellValue::Error(e) => write!(f, "#ERR: {}", e),
        }
    }
}

// ── 内部辅助 ─────────────────────────────────────────────────

/// 从 calamine::Data 转换
fn from_data(data: &Data) -> CellValue {
    match data {
        Data::Empty => CellValue::Empty,
        Data::String(s) => CellValue::String(s.clone()),
        Data::Float(f) => {
            if *f == (*f as i64) as f64 {
                CellValue::Int(*f as i64)
            } else {
                CellValue::Float(*f)
            }
        }
        Data::Int(i) => CellValue::Int(*i),
        Data::Bool(b) => CellValue::Bool(*b),
        Data::Error(e) => CellValue::Error(format!("{:?}", e)),
        Data::DateTime(dt) => CellValue::Float(dt.as_f64()),
        Data::DateTimeIso(s) => CellValue::String(s.clone()),
        Data::DurationIso(s) => CellValue::String(s.clone()),
    }
}

/// CSV 字符串智能解析为 CellValue
///
/// 按优先级尝试：i64 → f64 → bool → 原始字符串，空串 → Empty
fn csv_parse_value(s: &str) -> CellValue {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return CellValue::Empty;
    }
    // i64
    if let Ok(i) = trimmed.parse::<i64>() {
        return CellValue::Int(i);
    }
    // f64
    if let Ok(f) = trimmed.parse::<f64>() {
        return CellValue::Float(f);
    }
    // bool
    match trimmed.to_lowercase().as_str() {
        "true" | "yes" | "1" => return CellValue::Bool(true),
        "false" | "no" | "0" => return CellValue::Bool(false),
        _ => {}
    }
    CellValue::String(trimmed.to_string())
}

/// 从文件扩展名判断是否为 CSV
fn is_csv_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("csv"))
        .unwrap_or(false)
}

/// CSV 字节数据 → Vec<Vec<CellValue>>
fn csv_read_bytes(data: &[u8]) -> crate::Result<Vec<Vec<CellValue>>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(data);
    let mut rows = Vec::new();
    for result in rdr.records() {
        let record =
            result.map_err(|e| crate::Error::custom(51604, &format!("CSV parse error: {}", e)))?;
        let row: Vec<CellValue> = record.iter().map(csv_parse_value).collect();
        rows.push(row);
    }
    Ok(rows)
}

/// CSV 文件路径 → Vec<Vec<CellValue>>
fn csv_read_path<P: AsRef<Path>>(path: P) -> crate::Result<Vec<Vec<CellValue>>> {
    let data = std::fs::read(path.as_ref())
        .map_err(|e| crate::Error::custom(51601, &format!("Failed to read CSV file: {}", e)))?;
    csv_read_bytes(&data)
}

/// CellValue → CSV 输出字符串
fn csv_cell_to_string(cell: &CellValue) -> String {
    match cell {
        CellValue::Empty => String::new(),
        CellValue::String(s) => s.clone(),
        CellValue::Float(f) => format!("{}", f),
        CellValue::Int(i) => i.to_string(),
        CellValue::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        CellValue::Error(e) => format!("#ERR: {}", e),
    }
}

/// Vec<Vec<CellValue>> → CSV 字节
fn csv_write_bytes(data: &[Vec<CellValue>]) -> crate::Result<Vec<u8>> {
    let mut wtr = csv::WriterBuilder::new().from_writer(Vec::new());
    for row in data {
        let fields: Vec<String> = row.iter().map(csv_cell_to_string).collect();
        wtr.write_record(&fields)
            .map_err(|e| crate::Error::custom(51603, &format!("CSV write error: {}", e)))?;
    }
    wtr.flush()
        .map_err(|e| crate::Error::custom(51603, &format!("CSV flush error: {}", e)))?;
    let bytes = wtr
        .into_inner()
        .map_err(|e| crate::Error::custom(51603, &format!("CSV inner error: {}", e)))?;
    Ok(bytes)
}

// ═══════════════════════════════════════════════════════════════
//  Excel / CSV 读写
// ═══════════════════════════════════════════════════════════════

/// 表格工具
///
/// 支持读写 Excel (.xlsx / .xls / .xlsb / .ods) 和 CSV (.csv)。
///
/// **自动识别**：`read_path` 根据文件扩展名自动选择解析器：
/// - `.csv` → CSV 解析器（`csv` crate）
/// - `.xlsx` / `.xls` / `.xlsb` / `.ods` → Excel 解析器（`calamine`）
///
/// **单 Sheet**：当前所有 API 操作单个工作表。Excel 可通过 `sheet` 参数选择 sheet，
/// CSV 无 sheet 概念（忽略此参数）。
///
/// **大文件说明**：当前实现将全部数据加载到内存。超大文件建议使用 `csv::Reader`
/// 直接逐行读取，或对 Excel 使用 `calamine` 的流式 API。
///
/// ```no_run
/// use afaster::{Excel, CellValue, Format};
///
/// // 自动识别读取
/// let data = Excel::read_path("data.csv", None).unwrap();
/// let data = Excel::read_path("data.xlsx", None).unwrap();
///
/// // 写入 CSV
/// let bytes = Excel::write_as(&[
///     &["Name", "Age"],
///     &["Alice", "25"],
/// ], Format::Csv).unwrap();
/// ```
pub struct Excel;

impl Excel {
    // ── 读取（自动识别格式）─────────────────────────────────

    /// 从文件路径读取（自动识别 CSV / Excel）
    ///
    /// - `.csv` 扩展名 → CSV 解析
    /// - `.xlsx` / `.xls` / `.xlsb` / `.ods` → Excel 解析
    /// - `sheet`: 工作表名（CSV 忽略此参数）
    pub fn read_path<P: AsRef<Path>>(
        path: P,
        sheet: Option<&str>,
    ) -> crate::Result<Vec<Vec<CellValue>>> {
        if is_csv_extension(path.as_ref()) {
            return csv_read_path(path);
        }
        Self::read_xlsx_path(path, sheet)
    }

    /// 从字节数据读取（默认 Excel 优先）
    ///
    /// - `sheet`: 工作表名（CSV 忽略此参数）
    ///
    /// 如需指定格式或自动检测，使用 [`read_bytes_as`]
    pub fn read_bytes(data: &[u8], sheet: Option<&str>) -> crate::Result<Vec<Vec<CellValue>>> {
        Self::read_bytes_as(data, None, sheet)
    }

    /// 从字节数据读取（指定或自动检测格式）
    ///
    /// - `format`: `Some(Format::Csv)` 强制 CSV，`Some(Format::Xlsx)` 强制 Excel，
    ///   `None` 自动检测（先尝试 Excel，失败则 CSV）
    /// - `sheet`: 工作表名（CSV 忽略此参数）
    pub fn read_bytes_as(
        data: &[u8],
        format: Option<Format>,
        sheet: Option<&str>,
    ) -> crate::Result<Vec<Vec<CellValue>>> {
        match format {
            Some(Format::Csv) => return csv_read_bytes(data),
            Some(Format::Xlsx) => return Self::read_xlsx_bytes(data, sheet),
            None => {}
        }
        // 自动检测：先尝试 Excel，失败则 CSV
        match Self::read_xlsx_bytes(data, sheet) {
            Ok(rows) => Ok(rows),
            Err(_) => csv_read_bytes(data),
        }
    }

    /// 读取所有工作表（CSV 返回单个 sheet，key 为 "Sheet1"）
    pub fn read_all<P: AsRef<Path>>(
        path: P,
    ) -> crate::Result<HashMap<String, Vec<Vec<CellValue>>>> {
        if is_csv_extension(path.as_ref()) {
            let data = csv_read_path(path)?;
            let mut map = HashMap::new();
            map.insert("Sheet1".to_string(), data);
            return Ok(map);
        }
        Self::read_xlsx_all(path)
    }

    /// 读取并跳过前 N 行
    pub fn read_skip<P: AsRef<Path>>(
        path: P,
        sheet: Option<&str>,
        skip: usize,
    ) -> crate::Result<Vec<Vec<CellValue>>> {
        let data = Self::read_path(path, sheet)?;
        Ok(data.into_iter().skip(skip).collect())
    }

    /// 读取第一行作为表头，返回 (headers, rows)
    pub fn read_with_headers<P: AsRef<Path>>(
        path: P,
        sheet: Option<&str>,
    ) -> crate::Result<(Vec<String>, Vec<Vec<CellValue>>)> {
        let data = Self::read_path(path, sheet)?;
        if data.is_empty() {
            return Ok((vec![], vec![]));
        }
        let headers: Vec<String> = data[0].iter().map(|c| c.to_string()).collect();
        let rows = data.into_iter().skip(1).collect();
        Ok((headers, rows))
    }

    /// 读取为 JSON 数组（第一行作为 key）
    pub fn read_to_json<P: AsRef<Path>>(
        path: P,
        sheet: Option<&str>,
    ) -> crate::Result<Vec<serde_json::Value>> {
        let (headers, rows) = Self::read_with_headers(path, sheet)?;
        let mut result = Vec::new();
        for row in rows {
            let mut map = serde_json::Map::new();
            for (i, header) in headers.iter().enumerate() {
                let value = row
                    .get(i)
                    .map(|c| c.to_json())
                    .unwrap_or(serde_json::Value::Null);
                map.insert(header.clone(), value);
            }
            result.push(serde_json::Value::Object(map));
        }
        Ok(result)
    }

    // ── 读取（显式指定格式）─────────────────────────────────

    /// 显式从 CSV 文件读取
    pub fn read_csv_path<P: AsRef<Path>>(path: P) -> crate::Result<Vec<Vec<CellValue>>> {
        csv_read_path(path)
    }

    /// 显式从 CSV 字节读取
    pub fn read_csv_bytes(data: &[u8]) -> crate::Result<Vec<Vec<CellValue>>> {
        csv_read_bytes(data)
    }

    /// 显式从 Excel 文件读取
    pub fn read_xlsx_path<P: AsRef<Path>>(
        path: P,
        sheet: Option<&str>,
    ) -> crate::Result<Vec<Vec<CellValue>>> {
        let mut workbook: Xlsx<_> = open_workbook(path).map_err(|e| {
            crate::Error::custom(51601, &format!("Failed to open Excel file: {}", e))
        })?;

        let sheet_name = match sheet {
            Some(name) => name.to_string(),
            None => {
                let names = workbook.sheet_names();
                if names.is_empty() {
                    return Err(crate::Error::custom(51602, "Excel file has no sheets"));
                }
                names[0].clone()
            }
        };

        let range = workbook
            .worksheet_range(&sheet_name)
            .map_err(|e| crate::Error::custom(51602, &format!("Sheet not found: {}", e)))?;

        Ok(range
            .rows()
            .map(|row| row.iter().map(from_data).collect())
            .collect())
    }

    /// 显式从 Excel 字节读取
    pub fn read_xlsx_bytes(data: &[u8], sheet: Option<&str>) -> crate::Result<Vec<Vec<CellValue>>> {
        let cursor = Cursor::new(data);
        let mut workbook: Xlsx<_> = calamine::open_workbook_from_rs(cursor).map_err(|e| {
            crate::Error::custom(51601, &format!("Failed to parse Excel data: {}", e))
        })?;

        let sheet_name = match sheet {
            Some(name) => name.to_string(),
            None => {
                let names = workbook.sheet_names();
                if names.is_empty() {
                    return Err(crate::Error::custom(51602, "Excel file has no sheets"));
                }
                names[0].clone()
            }
        };

        let range = workbook
            .worksheet_range(&sheet_name)
            .map_err(|e| crate::Error::custom(51602, &format!("Sheet not found: {}", e)))?;

        Ok(range
            .rows()
            .map(|row| row.iter().map(from_data).collect())
            .collect())
    }

    /// 显式从 Excel 读取所有 sheet
    pub fn read_xlsx_all<P: AsRef<Path>>(
        path: P,
    ) -> crate::Result<HashMap<String, Vec<Vec<CellValue>>>> {
        let mut workbook: Xlsx<_> = open_workbook(path).map_err(|e| {
            crate::Error::custom(51601, &format!("Failed to open Excel file: {}", e))
        })?;

        let mut result = HashMap::new();
        let names = workbook.sheet_names().to_vec();

        for name in names {
            let range = workbook
                .worksheet_range(&name)
                .map_err(|e| crate::Error::custom(51602, &format!("Sheet error: {}", e)))?;

            let data = range
                .rows()
                .map(|row| row.iter().map(from_data).collect())
                .collect();
            result.insert(name, data);
        }

        Ok(result)
    }

    // ── 写入（默认 XLSX）────────────────────────────────────

    /// 写入 Excel（最简用法，默认 .xlsx）
    pub fn write(data: &[&[&str]]) -> crate::Result<Vec<u8>> {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let sheet = workbook.add_worksheet();

        for (row_idx, row) in data.iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                sheet
                    .write_string(row_idx as u32, col_idx as u16, *cell)
                    .map_err(|e| {
                        crate::Error::custom(51603, &format!("Write cell failed: {}", e))
                    })?;
            }
        }

        let buffer = workbook
            .save_to_buffer()
            .map_err(|e| crate::Error::custom(51603, &format!("Save Excel failed: {}", e)))?;

        Ok(buffer)
    }

    /// 写入 Excel（带类型，支持数字/字符串/布尔值）
    pub fn write_typed(
        data: &[Vec<CellValue>],
        sheet_name: Option<&str>,
    ) -> crate::Result<Vec<u8>> {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let sheet = match sheet_name {
            Some(name) => workbook.add_worksheet().set_name(name).map_err(|e| {
                crate::Error::custom(51603, &format!("Set sheet name failed: {}", e))
            })?,
            None => workbook.add_worksheet(),
        };

        Self::write_typed_to_sheet(sheet, data)?;

        let buffer = workbook
            .save_to_buffer()
            .map_err(|e| crate::Error::custom(51603, &format!("Save Excel failed: {}", e)))?;

        Ok(buffer)
    }

    /// 写入 Excel（带表头 + 数据行）
    pub fn write_with_headers(
        headers: &[&str],
        rows: &[Vec<CellValue>],
        sheet_name: Option<&str>,
    ) -> crate::Result<Vec<u8>> {
        let mut all_data = Vec::with_capacity(rows.len() + 1);
        let header_row: Vec<CellValue> = headers
            .iter()
            .map(|h| CellValue::String(h.to_string()))
            .collect();
        all_data.push(header_row);
        all_data.extend_from_slice(rows);
        Self::write_typed(&all_data, sheet_name)
    }

    /// 写入多工作表
    pub fn write_multi(sheets: &[(&str, Vec<Vec<CellValue>>)]) -> crate::Result<Vec<u8>> {
        let mut workbook = rust_xlsxwriter::Workbook::new();

        for (sheet_name, data) in sheets {
            let sheet = workbook
                .add_worksheet()
                .set_name(*sheet_name)
                .map_err(|e| {
                    crate::Error::custom(51603, &format!("Set sheet name failed: {}", e))
                })?;
            Self::write_typed_to_sheet(sheet, data)?;
        }

        let buffer = workbook
            .save_to_buffer()
            .map_err(|e| crate::Error::custom(51603, &format!("Save Excel failed: {}", e)))?;

        Ok(buffer)
    }

    // ── 写入（指定格式）─────────────────────────────────────

    /// 按指定格式写入（纯字符串）
    ///
    /// ```no_run
    /// use afaster::{Excel, Format};
    ///
    /// let bytes = Excel::write_as(&[
    ///     &["Name", "Age"],
    ///     &["Alice", "25"],
    /// ], Format::Csv).unwrap();
    /// ```
    pub fn write_as(data: &[&[&str]], format: Format) -> crate::Result<Vec<u8>> {
        match format {
            Format::Xlsx => Self::write(data),
            Format::Csv => {
                let typed: Vec<Vec<CellValue>> = data
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|s| CellValue::String(s.to_string()))
                            .collect()
                    })
                    .collect();
                csv_write_bytes(&typed)
            }
        }
    }

    /// 按指定格式写入（带类型）
    ///
    /// ```no_run
    /// use afaster::{Excel, CellValue, Format};
    ///
    /// let data = vec![
    ///     vec![CellValue::String("Name".into()), CellValue::String("Age".into())],
    ///     vec![CellValue::String("Alice".into()), CellValue::Int(25)],
    /// ];
    /// let bytes = Excel::write_typed_as(&data, Format::Csv, None).unwrap();
    /// ```
    pub fn write_typed_as(
        data: &[Vec<CellValue>],
        format: Format,
        sheet_name: Option<&str>,
    ) -> crate::Result<Vec<u8>> {
        match format {
            Format::Xlsx => Self::write_typed(data, sheet_name),
            Format::Csv => csv_write_bytes(data),
        }
    }

    /// 按指定格式写入（带表头）
    pub fn write_with_headers_as(
        headers: &[&str],
        rows: &[Vec<CellValue>],
        format: Format,
        sheet_name: Option<&str>,
    ) -> crate::Result<Vec<u8>> {
        match format {
            Format::Xlsx => Self::write_with_headers(headers, rows, sheet_name),
            Format::Csv => {
                let mut all_data = Vec::with_capacity(rows.len() + 1);
                let header_row: Vec<CellValue> = headers
                    .iter()
                    .map(|h| CellValue::String(h.to_string()))
                    .collect();
                all_data.push(header_row);
                all_data.extend_from_slice(rows);
                csv_write_bytes(&all_data)
            }
        }
    }

    /// 按指定格式写入（多 sheet，CSV 仅写第一个 sheet）
    pub fn write_multi_as(
        sheets: &[(&str, Vec<Vec<CellValue>>)],
        format: Format,
    ) -> crate::Result<Vec<u8>> {
        match format {
            Format::Xlsx => Self::write_multi(sheets),
            Format::Csv => {
                if let Some((_name, data)) = sheets.first() {
                    csv_write_bytes(data)
                } else {
                    Ok(Vec::new())
                }
            }
        }
    }

    // ── 内部工具 ─────────────────────────────────────────────

    /// 将带类型数据写入 worksheet（复用于 write_typed 和 write_multi）
    fn write_typed_to_sheet(
        sheet: &mut rust_xlsxwriter::Worksheet,
        data: &[Vec<CellValue>],
    ) -> crate::Result<()> {
        for (row_idx, row) in data.iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                let r = row_idx as u32;
                let c = col_idx as u16;
                match cell {
                    CellValue::Empty => {}
                    CellValue::String(s) => {
                        sheet.write_string(r, c, s.as_str()).map_err(|e| {
                            crate::Error::custom(51603, &format!("Write failed: {}", e))
                        })?;
                    }
                    CellValue::Float(f) => {
                        sheet.write_number(r, c, *f).map_err(|e| {
                            crate::Error::custom(51603, &format!("Write failed: {}", e))
                        })?;
                    }
                    CellValue::Int(i) => {
                        sheet.write_number(r, c, *i as f64).map_err(|e| {
                            crate::Error::custom(51603, &format!("Write failed: {}", e))
                        })?;
                    }
                    CellValue::Bool(b) => {
                        sheet.write_boolean(r, c, *b).map_err(|e| {
                            crate::Error::custom(51603, &format!("Write failed: {}", e))
                        })?;
                    }
                    CellValue::Error(e) => {
                        sheet
                            .write_string(r, c, format!("#ERR: {}", e))
                            .map_err(|e| {
                                crate::Error::custom(51603, &format!("Write failed: {}", e))
                            })?;
                    }
                }
            }
        }
        Ok(())
    }

    /// 将 CellValue 行写入 worksheet（复用于 RowWriter）
    fn write_row_to_sheet(
        sheet: &mut rust_xlsxwriter::Worksheet,
        row_idx: u32,
        row: &[CellValue],
    ) -> crate::Result<()> {
        for (col_idx, cell) in row.iter().enumerate() {
            let c = col_idx as u16;
            match cell {
                CellValue::Empty => {}
                CellValue::String(s) => {
                    sheet.write_string(row_idx, c, s.as_str()).map_err(|e| {
                        crate::Error::custom(51603, &format!("Write failed: {}", e))
                    })?;
                }
                CellValue::Float(f) => {
                    sheet.write_number(row_idx, c, *f).map_err(|e| {
                        crate::Error::custom(51603, &format!("Write failed: {}", e))
                    })?;
                }
                CellValue::Int(i) => {
                    sheet.write_number(row_idx, c, *i as f64).map_err(|e| {
                        crate::Error::custom(51603, &format!("Write failed: {}", e))
                    })?;
                }
                CellValue::Bool(b) => {
                    sheet.write_boolean(row_idx, c, *b).map_err(|e| {
                        crate::Error::custom(51603, &format!("Write failed: {}", e))
                    })?;
                }
                CellValue::Error(e) => {
                    sheet
                        .write_string(row_idx, c, format!("#ERR: {}", e))
                        .map_err(|e| {
                            crate::Error::custom(51603, &format!("Write failed: {}", e))
                        })?;
                }
            }
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════
//  流式读取 RowReader
// ═══════════════════════════════════════════════════════════════

/// CSV 流式读取器（文件）
struct CsvStreamReader {
    reader: csv::Reader<BufReader<File>>,
}

/// CSV 流式读取器（字节）
struct CsvBytesReader {
    reader: csv::Reader<Cursor<Vec<u8>>>,
}

/// Excel 行迭代器数据
struct XlsxIterData {
    names: Vec<String>,
    selected: String,
    range: calamine::Range<Data>,
}

/// 流式行读取器
///
/// 逐行读取表格数据，避免一次性加载全部内容到内存。
///
/// - **CSV**：真正流式读取，逐行从文件/字节流解析
/// - **Excel**：内部加载整个 sheet（calamine 限制），但提供一致的逐行迭代接口
///
/// ```no_run
/// use afaster::RowReader;
///
/// // 流式读取 CSV
/// let mut reader = RowReader::from_path("large.csv", None).unwrap();
/// while let Some(row) = reader.next_row() {
///     let row = row.unwrap();
///     println!("{:?}", row);
/// }
///
/// // 带表头读取
/// let mut reader = RowReader::from_path("data.csv", None).unwrap();
/// let headers = reader.headers().unwrap();
/// while let Some(row) = reader.next_row() {
///     // ...
/// }
/// ```
pub struct RowReader {
    /// 表头行（可选，通过 `headers()` 提取第一行后设置）
    headers: Option<Vec<String>>,
    /// CSV 文件读取器
    csv_file: Option<CsvStreamReader>,
    /// CSV 字节读取器
    csv_bytes: Option<CsvBytesReader>,
    /// Excel 迭代器数据
    xlsx: Option<XlsxIterData>,
    /// Excel 当前行号
    xlsx_row_idx: usize,
    /// 总行数（用于进度）
    total_rows: Option<usize>,
}

impl RowReader {
    /// 从文件路径创建流式读取器（自动识别格式）
    ///
    /// - `.csv` → CSV 流式读取
    /// - `.xlsx` / `.xls` / `.xlsb` / `.ods` → Excel（加载后迭代）
    /// - `sheet`: Excel 工作表名（CSV 忽略）
    pub fn from_path<P: AsRef<Path>>(path: P, sheet: Option<&str>) -> crate::Result<Self> {
        if is_csv_extension(path.as_ref()) {
            return Self::from_csv_path(path);
        }
        Self::from_xlsx_path(path, sheet)
    }

    /// 从 CSV 文件创建流式读取器
    pub fn from_csv_path<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        let file = File::open(path.as_ref())
            .map_err(|e| crate::Error::custom(51601, &format!("Failed to open CSV file: {}", e)))?;
        let reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(BufReader::new(file));
        Ok(Self {
            headers: None,
            csv_file: Some(CsvStreamReader { reader }),
            csv_bytes: None,
            xlsx: None,
            xlsx_row_idx: 0,
            total_rows: None,
        })
    }

    /// 从 CSV 字节创建流式读取器
    pub fn from_csv_bytes(data: Vec<u8>) -> crate::Result<Self> {
        let reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(Cursor::new(data));
        Ok(Self {
            headers: None,
            csv_file: None,
            csv_bytes: Some(CsvBytesReader { reader }),
            xlsx: None,
            xlsx_row_idx: 0,
            total_rows: None,
        })
    }

    /// 从 Excel 文件创建读取器
    pub fn from_xlsx_path<P: AsRef<Path>>(path: P, sheet: Option<&str>) -> crate::Result<Self> {
        let mut workbook: Xlsx<BufReader<File>> = open_workbook(path).map_err(|e| {
            crate::Error::custom(51601, &format!("Failed to open Excel file: {}", e))
        })?;
        let names = workbook.sheet_names();
        let sheet_name = match sheet {
            Some(name) => name.to_string(),
            None => {
                if names.is_empty() {
                    return Err(crate::Error::custom(51602, "Excel file has no sheets"));
                }
                names[0].clone()
            }
        };
        let range = workbook
            .worksheet_range(&sheet_name)
            .map_err(|e| crate::Error::custom(51602, &format!("Sheet not found: {}", e)))?;
        let total = range.rows().len();
        Ok(Self {
            headers: None,
            csv_file: None,
            csv_bytes: None,
            xlsx: Some(XlsxIterData {
                names,
                selected: sheet_name,
                range,
            }),
            xlsx_row_idx: 0,
            total_rows: Some(total),
        })
    }

    /// 从 Excel 字节创建读取器
    pub fn from_xlsx_bytes(data: &[u8], sheet: Option<&str>) -> crate::Result<Self> {
        let cursor = Cursor::new(data.to_vec());
        let mut workbook: Xlsx<Cursor<Vec<u8>>> =
            calamine::open_workbook_from_rs(cursor).map_err(|e| {
                crate::Error::custom(51601, &format!("Failed to parse Excel data: {}", e))
            })?;
        let names = workbook.sheet_names();
        let sheet_name = match sheet {
            Some(name) => name.to_string(),
            None => {
                if names.is_empty() {
                    return Err(crate::Error::custom(51602, "Excel file has no sheets"));
                }
                names[0].clone()
            }
        };
        let range = workbook
            .worksheet_range(&sheet_name)
            .map_err(|e| crate::Error::custom(51602, &format!("Sheet not found: {}", e)))?;
        let total = range.rows().len();
        Ok(Self {
            headers: None,
            csv_file: None,
            csv_bytes: None,
            xlsx: Some(XlsxIterData {
                names,
                selected: sheet_name,
                range,
            }),
            xlsx_row_idx: 0,
            total_rows: Some(total),
        })
    }

    /// 提取第一行作为表头，后续 `next_row()` 从第二行开始
    ///
    /// 返回 `None` 如果没有数据
    pub fn headers(&mut self) -> Option<Vec<String>> {
        if self.headers.is_some() {
            return self.headers.clone();
        }
        match self.next_row() {
            Some(Ok(row)) => {
                let h: Vec<String> = row.iter().map(|c| c.to_string()).collect();
                self.headers = Some(h.clone());
                Some(h)
            }
            _ => None,
        }
    }

    /// 读取下一行
    ///
    /// 返回 `Some(Ok(row))` 有数据，`Some(Err(e))` 解析错误，`None` 结束
    pub fn next_row(&mut self) -> Option<crate::Result<Vec<CellValue>>> {
        // CSV 文件
        if let Some(ref mut csv) = self.csv_file {
            return match csv.reader.records().next() {
                Some(Ok(record)) => {
                    let row: Vec<CellValue> = record.iter().map(csv_parse_value).collect();
                    Some(Ok(row))
                }
                Some(Err(e)) => Some(Err(crate::Error::custom(
                    51604,
                    &format!("CSV parse error: {}", e),
                ))),
                None => None,
            };
        }
        // CSV 字节
        if let Some(ref mut csv) = self.csv_bytes {
            return match csv.reader.records().next() {
                Some(Ok(record)) => {
                    let row: Vec<CellValue> = record.iter().map(csv_parse_value).collect();
                    Some(Ok(row))
                }
                Some(Err(e)) => Some(Err(crate::Error::custom(
                    51604,
                    &format!("CSV parse error: {}", e),
                ))),
                None => None,
            };
        }
        // Excel
        if let Some(ref xlsx) = self.xlsx {
            let rows: Vec<&[Data]> = xlsx.range.rows().collect();
            if self.xlsx_row_idx < rows.len() {
                let row: Vec<CellValue> = rows[self.xlsx_row_idx].iter().map(from_data).collect();
                self.xlsx_row_idx += 1;
                return Some(Ok(row));
            }
        }
        None
    }

    /// 获取总行数（仅 Excel 可用，CSV 返回 None）
    pub fn total_rows(&self) -> Option<usize> {
        self.total_rows
    }

    /// 获取 Excel 工作表名列表（仅 Excel 可用）
    pub fn sheet_names(&self) -> Option<&[String]> {
        self.xlsx.as_ref().map(|x| x.names.as_slice())
    }

    /// 获取当前选中的工作表名（仅 Excel 可用）
    pub fn selected_sheet(&self) -> Option<&str> {
        self.xlsx.as_ref().map(|x| x.selected.as_str())
    }

    /// 将剩余行全部收集到 Vec（跳过已读取的行）
    pub fn collect_remaining(&mut self) -> crate::Result<Vec<Vec<CellValue>>> {
        let mut rows = Vec::new();
        while let Some(result) = self.next_row() {
            rows.push(result?);
        }
        Ok(rows)
    }
}

/// 实现 Iterator trait，支持 `for row in reader { ... }` 语法
///
/// ```no_run
/// use afaster::RowReader;
///
/// let reader = RowReader::from_path("data.csv", None).unwrap();
/// for row in reader {
///     match row {
///         Ok(cells) => println!("{:?}", cells),
///         Err(e) => eprintln!("Error: {}", e),
///     }
/// }
/// ```
impl Iterator for RowReader {
    type Item = crate::Result<Vec<CellValue>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_row()
    }
}

// ═══════════════════════════════════════════════════════════════
//  流式写入 RowWriter
// ═══════════════════════════════════════════════════════════════

/// 流式行写入器
///
/// 逐行写入表格数据，最后通过 `finish_*` 方法一次性生成文件或字节。
///
/// - 写入 CSV 时直接输出到文件/Writer，不在内存中积累全部数据
/// - 写入 Excel 时在内存中积累数据（xlsx 格式要求），最终一次性写出
///
/// ```no_run
/// use afaster::{RowWriter, CellValue, Format};
///
/// // 流式写入 CSV 文件
/// let mut writer = RowWriter::new();
/// writer.write_row(&[CellValue::String("Name".into()), CellValue::String("Age".into())]);
/// writer.write_row(&[CellValue::String("Alice".into()), CellValue::Int(25)]);
/// writer.write_row(&[CellValue::String("Bob".into()), CellValue::Int(30)]);
/// writer.finish_csv_path("output.csv").unwrap();
///
/// // 流式写入 Excel 文件
/// let mut writer = RowWriter::new();
/// writer.write_row(&[CellValue::String("Name".into()), CellValue::String("Age".into())]);
/// writer.write_row(&[CellValue::String("Alice".into()), CellValue::Int(25)]);
/// writer.finish_xlsx_path("output.xlsx", None).unwrap();
///
/// // 生成字节
/// let mut writer = RowWriter::new();
/// writer.write_row(&[CellValue::String("Hello".into())]);
/// let bytes = writer.finish_bytes(Format::Csv).unwrap();
/// ```
pub struct RowWriter {
    rows: Vec<Vec<CellValue>>,
    /// CSV 直接写入文件的 Writer（仅在 finish_csv_path 时设置）
    csv_file_writer: Option<csv::Writer<BufWriter<File>>>,
    csv_file_rows: usize,
}

impl RowWriter {
    /// 创建新的流式写入器
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            csv_file_writer: None,
            csv_file_rows: 0,
        }
    }

    /// 创建直接写入 CSV 文件的写入器（边写边输出，不积累内存）
    ///
    /// ```no_run
    /// use afaster::{RowWriter, CellValue};
    ///
    /// let mut writer = RowWriter::from_csv_path("output.csv").unwrap();
    /// writer.write_row(&[CellValue::String("Name".into()), CellValue::String("Age".into())]);
    /// writer.write_row(&[CellValue::String("Alice".into()), CellValue::Int(25)]);
    /// writer.finish().unwrap(); // 关闭文件
    /// ```
    pub fn from_csv_path<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        let file = File::create(path.as_ref()).map_err(|e| {
            crate::Error::custom(51603, &format!("Failed to create CSV file: {}", e))
        })?;
        let writer = csv::WriterBuilder::new().from_writer(BufWriter::new(file));
        Ok(Self {
            rows: Vec::new(),
            csv_file_writer: Some(writer),
            csv_file_rows: 0,
        })
    }

    /// 写入一行数据
    ///
    /// 如果是通过 `from_csv_path` 创建的写入器，数据会立即写入文件。
    /// 否则数据积累在内存中，等待 `finish_*` 方法处理。
    pub fn write_row(&mut self, row: &[CellValue]) -> crate::Result<()> {
        // 如果有 CSV 文件写入器，直接写入
        if let Some(ref mut wtr) = self.csv_file_writer {
            let fields: Vec<String> = row.iter().map(csv_cell_to_string).collect();
            wtr.write_record(&fields)
                .map_err(|e| crate::Error::custom(51603, &format!("CSV write error: {}", e)))?;
            self.csv_file_rows += 1;
            return Ok(());
        }
        // 否则积累到内存
        self.rows.push(row.to_vec());
        Ok(())
    }

    /// 写入一行纯字符串数据
    pub fn write_row_str(&mut self, row: &[&str]) -> crate::Result<()> {
        let cells: Vec<CellValue> = row
            .iter()
            .map(|s| CellValue::String(s.to_string()))
            .collect();
        self.write_row(&cells)
    }

    /// 当前已写入的行数
    pub fn len(&self) -> usize {
        if self.csv_file_writer.is_some() {
            self.csv_file_rows
        } else {
            self.rows.len()
        }
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 完成写入并关闭文件（仅用于 `from_csv_path` 创建的写入器）
    pub fn finish(&mut self) -> crate::Result<()> {
        if let Some(mut wtr) = self.csv_file_writer.take() {
            wtr.flush()
                .map_err(|e| crate::Error::custom(51603, &format!("CSV flush error: {}", e)))?;
        }
        Ok(())
    }

    /// 完成写入，生成 CSV 字节
    pub fn finish_csv_bytes(&self) -> crate::Result<Vec<u8>> {
        csv_write_bytes(&self.rows)
    }

    /// 完成写入，生成 Excel 字节
    pub fn finish_xlsx_bytes(&self, sheet_name: Option<&str>) -> crate::Result<Vec<u8>> {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let sheet = match sheet_name {
            Some(name) => workbook.add_worksheet().set_name(name).map_err(|e| {
                crate::Error::custom(51603, &format!("Set sheet name failed: {}", e))
            })?,
            None => workbook.add_worksheet(),
        };

        for (row_idx, row) in self.rows.iter().enumerate() {
            Excel::write_row_to_sheet(sheet, row_idx as u32, row)?;
        }

        workbook
            .save_to_buffer()
            .map_err(|e| crate::Error::custom(51603, &format!("Save Excel failed: {}", e)))
    }

    /// 完成写入，生成指定格式的字节
    pub fn finish_bytes(&self, format: Format) -> crate::Result<Vec<u8>> {
        match format {
            Format::Xlsx => self.finish_xlsx_bytes(None),
            Format::Csv => self.finish_csv_bytes(),
        }
    }

    /// 完成写入，保存到 CSV 文件
    pub fn finish_csv_path<P: AsRef<Path>>(&self, path: P) -> crate::Result<()> {
        let bytes = self.finish_csv_bytes()?;
        std::fs::write(path, bytes)
            .map_err(|e| crate::Error::custom(51603, &format!("Write CSV file failed: {}", e)))
    }

    /// 完成写入，保存到 Excel 文件
    pub fn finish_xlsx_path<P: AsRef<Path>>(
        &self,
        path: P,
        sheet_name: Option<&str>,
    ) -> crate::Result<()> {
        let bytes = self.finish_xlsx_bytes(sheet_name)?;
        std::fs::write(path, bytes)
            .map_err(|e| crate::Error::custom(51603, &format!("Write Excel file failed: {}", e)))
    }

    /// 完成写入，保存到指定格式的文件
    pub fn finish_path<P: AsRef<Path>>(&self, path: P, format: Format) -> crate::Result<()> {
        match format {
            Format::Xlsx => self.finish_xlsx_path(path, None),
            Format::Csv => self.finish_csv_path(path),
        }
    }
}
