//! TraceStore 的 SQLite 实现

use super::err;
use super::store::*;
use sqlx::Row;
use sqlx::sqlite::SqlitePool;

/// SQLite 存储实现
#[derive(Clone)]
pub struct SqliteTraceStore {
    pool: SqlitePool,
}

impl SqliteTraceStore {
    /// 创建并初始化 SQLite 存储
    pub async fn new(db_path: &str) -> crate::Result<Self> {
        let url = format!("sqlite:{}?mode=rwc", db_path);
        let pool = SqlitePool::connect(&url)
            .await
            .map_err(|e| err::sqlite_init_failed(&format!("connect: {}", e)))?;

        // 启用 WAL 模式，避免并发写入时的 readonly 错误
        let _ = sqlx::query("PRAGMA journal_mode=WAL").execute(&pool).await;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS tracing_spans (
                trace_id        TEXT NOT NULL,
                span_id         TEXT NOT NULL,
                parent_span_id  TEXT,
                handler_name    TEXT NOT NULL,
                handler_desc    TEXT NOT NULL DEFAULT '',
                transport       TEXT NOT NULL DEFAULT 'http',
                is_binary       INTEGER NOT NULL DEFAULT 0,
                method          TEXT NOT NULL DEFAULT '',
                long_connection INTEGER NOT NULL DEFAULT 0,
                is_root         INTEGER NOT NULL DEFAULT 0,
                start_time      INTEGER NOT NULL,
                duration_us     INTEGER NOT NULL,
                status          TEXT NOT NULL DEFAULT 'ok',
                error_code      INTEGER,
                error_message   TEXT,
                service_name    TEXT NOT NULL DEFAULT '',
                PRIMARY KEY (trace_id, span_id)
            )",
        )
        .execute(&pool)
        .await
        .map_err(|e| err::sqlite_init_failed(&format!("create table: {}", e)))?;

        for sql in [
            "CREATE INDEX IF NOT EXISTS idx_tracing_spans_trace_id ON tracing_spans(trace_id)",
            "CREATE INDEX IF NOT EXISTS idx_tracing_spans_start_time ON tracing_spans(start_time)",
            "CREATE INDEX IF NOT EXISTS idx_tracing_spans_service ON tracing_spans(service_name)",
            "CREATE INDEX IF NOT EXISTS idx_tracing_spans_status ON tracing_spans(status)",
            "CREATE INDEX IF NOT EXISTS idx_tracing_spans_handler ON tracing_spans(handler_name)",
            "CREATE INDEX IF NOT EXISTS idx_tracing_spans_is_root ON tracing_spans(is_root)",
            "CREATE INDEX IF NOT EXISTS idx_tracing_spans_parent ON tracing_spans(trace_id, parent_span_id)",
        ] {
            sqlx::query(sql)
                .execute(&pool)
                .await
                .map_err(|e| err::sqlite_init_failed(&format!("create index: {}", e)))?;
        }

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

impl TraceStore for SqliteTraceStore {
    fn insert_span(
        &self,
        span: SpanData,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send + '_>> {
        Box::pin(async move {
            sqlx::query(
                "INSERT OR REPLACE INTO tracing_spans
                 (trace_id, span_id, parent_span_id, handler_name, handler_desc, transport,
                  is_binary, method, long_connection, is_root,
                  start_time, duration_us, status, error_code, error_message, service_name)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&span.trace_id)
            .bind(&span.span_id)
            .bind(&span.parent_span_id)
            .bind(&span.handler_name)
            .bind(&span.handler_desc)
            .bind(&span.transport)
            .bind(span.is_binary as i32)
            .bind(&span.method)
            .bind(span.long_connection as i32)
            .bind(span.is_root as i32)
            .bind(span.start_time)
            .bind(span.duration_us)
            .bind(span.status.to_string())
            .bind(span.error_code)
            .bind(&span.error_message)
            .bind(&span.service_name)
            .execute(&self.pool)
            .await
            .map_err(|e| err::sqlite_error(&format!("insert_span: {}", e)))?;
            Ok(())
        })
    }

    fn insert_spans(
        &self,
        spans: Vec<SpanData>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send + '_>> {
        Box::pin(async move {
            let mut tx = self
                .pool
                .begin()
                .await
                .map_err(|e| err::sqlite_error(&format!("begin tx: {}", e)))?;

            for span in spans {
                sqlx::query(
                    "INSERT OR REPLACE INTO tracing_spans
                     (trace_id, span_id, parent_span_id, handler_name, handler_desc, transport,
                      is_binary, method, long_connection, is_root,
                      start_time, duration_us, status, error_code, error_message, service_name)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&span.trace_id)
                .bind(&span.span_id)
                .bind(&span.parent_span_id)
                .bind(&span.handler_name)
                .bind(&span.handler_desc)
                .bind(&span.transport)
                .bind(span.is_binary as i32)
                .bind(&span.method)
                .bind(span.long_connection as i32)
                .bind(span.is_root as i32)
                .bind(span.start_time)
                .bind(span.duration_us)
                .bind(span.status.to_string())
                .bind(span.error_code)
                .bind(&span.error_message)
                .bind(&span.service_name)
                .execute(&mut *tx)
                .await
                .map_err(|e| err::sqlite_error(&format!("insert_span in batch: {}", e)))?;
            }

            tx.commit()
                .await
                .map_err(|e| err::sqlite_error(&format!("commit: {}", e)))?;
            Ok(())
        })
    }

    fn get_trace(
        &self,
        trace_id: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<Vec<SpanData>>> + Send + '_>,
    > {
        let trace_id = trace_id.to_string();
        Box::pin(async move {
            let rows = sqlx::query(
                "SELECT trace_id, span_id, parent_span_id, handler_name, handler_desc, transport,
                        is_binary, method, long_connection, is_root,
                        start_time, duration_us, status, error_code, error_message, service_name
                 FROM tracing_spans WHERE trace_id = ? ORDER BY start_time ASC",
            )
            .bind(&trace_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| err::sqlite_error(&format!("get_trace: {}", e)))?;

            Ok(rows.into_iter().map(row_to_span).collect())
        })
    }

    fn list_traces(
        &self,
        filter: TraceFilter,
        page: i64,
        page_size: i64,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<TraceListResult>> + Send + '_>,
    > {
        Box::pin(async move {
            // 只查 is_root=true 的 span
            let rows = sqlx::query(
                "SELECT trace_id, span_id, parent_span_id, handler_name, handler_desc, transport,
                        is_binary, method, long_connection, is_root,
                        start_time, duration_us, status, error_code, error_message, service_name
                 FROM tracing_spans WHERE is_root = 1 ORDER BY start_time DESC",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| err::sqlite_error(&format!("list_traces: {}", e)))?;

            let all_spans: Vec<SpanData> = rows.into_iter().map(row_to_span).collect();

            // 过滤
            let summaries: Vec<TraceSummary> = all_spans
                .into_iter()
                .filter_map(|span| {
                    if let Some(from) = filter.start_time_from {
                        if span.start_time < from {
                            return None;
                        }
                    }
                    if let Some(to) = filter.start_time_to {
                        if span.start_time > to {
                            return None;
                        }
                    }
                    if let Some(ref svc) = filter.service_name {
                        if span.service_name != *svc {
                            return None;
                        }
                    }
                    if let Some(ref tp) = filter.transport {
                        if !tp.contains(&span.transport) {
                            return None;
                        }
                    }
                    if let Some(ref status) = filter.status {
                        if span.status.to_string() != status.to_string() {
                            return None;
                        }
                    }
                    if let Some(min_dur) = filter.min_duration_us {
                        if span.duration_us < min_dur {
                            return None;
                        }
                    }
                    if let Some(max_dur) = filter.max_duration_us {
                        if span.duration_us > max_dur {
                            return None;
                        }
                    }
                    if let Some(ec) = filter.error_code {
                        if span.error_code != Some(ec) {
                            return None;
                        }
                    }
                    if let Some(ref handler) = filter.handler_name {
                        if span.handler_name != *handler {
                            return None;
                        }
                    }

                    Some(TraceSummary {
                        trace_id: span.trace_id,
                        span_id: span.span_id,
                        parent_span_id: span.parent_span_id,
                        root_handler: span.handler_name,
                        service_name: span.service_name,
                        transport: span.transport,
                        is_binary: span.is_binary,
                        method: span.method,
                        long_connection: span.long_connection,
                        start_time: span.start_time,
                        duration_us: span.duration_us,
                        span_count: 1,
                        status: span.status,
                        has_error: span.status == SpanStatus::Error,
                    })
                })
                .collect();

            let total = summaries.len() as i64;
            let offset = ((page - 1) * page_size) as usize;
            let data = summaries
                .into_iter()
                .skip(offset)
                .take(page_size as usize)
                .collect();

            Ok(TraceListResult {
                total,
                page,
                page_size,
                data,
            })
        })
    }

    fn stats(
        &self,
        filter: StatsFilter,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<TraceStats>> + Send + '_>>
    {
        Box::pin(async move {
            // 直接查询 root span
            // 排除：普通 ws、sse、ws-binary 中的 long、tcp 中的 long
            let rows = sqlx::query(
                "SELECT trace_id, span_id, parent_span_id, handler_name, handler_desc, transport,
                        is_binary, method, long_connection, is_root,
                        start_time, duration_us, status, error_code, error_message, service_name
                 FROM tracing_spans
                 WHERE is_root = 1
                   AND transport NOT IN ('ws', 'sse')
                   AND (long_connection = 0 OR transport NOT IN ('ws-binary', 'tcp'))",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| err::sqlite_error(&format!("stats: {}", e)))?;

            let root_spans: Vec<SpanData> = rows.into_iter().map(row_to_span).collect();

            let mut durations: Vec<i64> = Vec::new();
            let mut total_count: i64 = 0;
            let mut error_count: i64 = 0;

            for span in &root_spans {
                if let Some(from) = filter.start_time_from {
                    if span.start_time < from {
                        continue;
                    }
                }
                if let Some(to) = filter.start_time_to {
                    if span.start_time > to {
                        continue;
                    }
                }
                if let Some(ref svc) = filter.service_name {
                    if span.service_name != *svc {
                        continue;
                    }
                }
                if let Some(ref tp) = filter.transport {
                    if span.transport != *tp {
                        continue;
                    }
                }

                total_count += 1;
                if span.status == SpanStatus::Error {
                    error_count += 1;
                }
                if span.duration_us > 0 {
                    durations.push(span.duration_us);
                }
            }

            durations.sort();
            let dur_count = durations.len() as i64;

            if total_count == 0 {
                return Ok(TraceStats {
                    total_traces: 0,
                    error_traces: 0,
                    error_rate: 0.0,
                    avg_duration_us: 0.0,
                    p50_duration_us: 0,
                    p95_duration_us: 0,
                    p99_duration_us: 0,
                    max_duration_us: 0,
                    min_duration_us: 0,
                });
            }

            let (avg, p50, p95, p99, max, min) = if dur_count > 0 {
                let sum: i64 = durations.iter().sum();
                (
                    sum as f64 / dur_count as f64,
                    percentile(&durations, 50),
                    percentile(&durations, 95),
                    percentile(&durations, 99),
                    *durations.last().unwrap(),
                    *durations.first().unwrap(),
                )
            } else {
                (0.0, 0, 0, 0, 0, 0)
            };

            Ok(TraceStats {
                total_traces: total_count,
                error_traces: error_count,
                error_rate: (error_count as f64 / total_count as f64 * 10000.0).round() / 100.0,
                avg_duration_us: (avg * 100.0).round() / 100.0,
                p50_duration_us: p50,
                p95_duration_us: p95,
                p99_duration_us: p99,
                max_duration_us: max,
                min_duration_us: min,
            })
        })
    }

    fn cleanup(
        &self,
        before_ts: i64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<u64>> + Send + '_>> {
        Box::pin(async move {
            let result = sqlx::query("DELETE FROM tracing_spans WHERE start_time < ?")
                .bind(before_ts)
                .execute(&self.pool)
                .await
                .map_err(|e| err::sqlite_error(&format!("cleanup: {}", e)))?;
            Ok(result.rows_affected())
        })
    }

    fn get_children(
        &self,
        trace_id: &str,
        parent_span_id: &str,
        page: i64,
        page_size: i64,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<ChildrenResult>> + Send + '_>,
    > {
        let trace_id = trace_id.to_string();
        let parent_span_id = parent_span_id.to_string();
        Box::pin(async move {
            let count_row = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM tracing_spans WHERE trace_id = ? AND parent_span_id = ?",
            )
            .bind(&trace_id)
            .bind(&parent_span_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| err::sqlite_error(&format!("get_children count: {}", e)))?;

            let offset = (page - 1) * page_size;
            let rows = sqlx::query(
                "SELECT trace_id, span_id, parent_span_id, handler_name, handler_desc, transport,
                        is_binary, method, long_connection, is_root,
                        start_time, duration_us, status, error_code, error_message, service_name
                 FROM tracing_spans
                 WHERE trace_id = ? AND parent_span_id = ?
                 ORDER BY start_time DESC
                 LIMIT ? OFFSET ?",
            )
            .bind(&trace_id)
            .bind(&parent_span_id)
            .bind(page_size)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| err::sqlite_error(&format!("get_children: {}", e)))?;

            Ok(ChildrenResult {
                total: count_row,
                page,
                page_size,
                data: rows.into_iter().map(row_to_span).collect(),
            })
        })
    }

    fn get_span(
        &self,
        trace_id: &str,
        span_id: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<Option<SpanData>>> + Send + '_>,
    > {
        let trace_id = trace_id.to_string();
        let span_id = span_id.to_string();
        Box::pin(async move {
            let row = sqlx::query(
                "SELECT trace_id, span_id, parent_span_id, handler_name, handler_desc, transport,
                        is_binary, method, long_connection, is_root,
                        start_time, duration_us, status, error_code, error_message, service_name
                 FROM tracing_spans WHERE trace_id = ? AND span_id = ?",
            )
            .bind(&trace_id)
            .bind(&span_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| err::sqlite_error(&format!("get_span: {}", e)))?;

            Ok(row.map(row_to_span))
        })
    }
}

// ═══════════════════════════════════════════════════════════════
//  辅助函数
// ═══════════════════════════════════════════════════════════════

fn row_to_span(row: sqlx::sqlite::SqliteRow) -> SpanData {
    SpanData {
        trace_id: row.get("trace_id"),
        span_id: row.get("span_id"),
        parent_span_id: row.get("parent_span_id"),
        handler_name: row.get("handler_name"),
        handler_desc: row.get("handler_desc"),
        transport: row.get("transport"),
        is_binary: row.get::<i32, _>("is_binary") != 0,
        method: row.get("method"),
        long_connection: row.get::<i32, _>("long_connection") != 0,
        is_root: row.get::<i32, _>("is_root") != 0,
        start_time: row.get("start_time"),
        duration_us: row.get("duration_us"),
        status: {
            let s: String = row.get("status");
            if s == "error" {
                SpanStatus::Error
            } else {
                SpanStatus::Ok
            }
        },
        error_code: row.get("error_code"),
        error_message: row.get("error_message"),
        service_name: row.get("service_name"),
    }
}

fn percentile(sorted: &[i64], pct: i64) -> i64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((pct as f64 / 100.0) * sorted.len() as f64) as usize;
    let idx = idx.min(sorted.len() - 1);
    sorted[idx]
}
