use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    Layer, filter::filter_fn, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};

pub struct LogGuard {
    pub _error: tracing_appender::non_blocking::WorkerGuard,
    pub _warn: tracing_appender::non_blocking::WorkerGuard,
    pub _info: tracing_appender::non_blocking::WorkerGuard,
    pub _trace: tracing_appender::non_blocking::WorkerGuard,
}

pub fn init_logger() -> LogGuard {
    let error_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "error.log");
    let warn_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "warn.log");
    let info_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "info.log");
    let trace_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "trace.log");

    let (error_writer, error_guard) = tracing_appender::non_blocking(error_appender);
    let (warn_writer, warn_guard) = tracing_appender::non_blocking(warn_appender);
    let (info_writer, info_guard) = tracing_appender::non_blocking(info_appender);
    let (trace_writer, trace_guard) = tracing_appender::non_blocking(trace_appender);

    // 只记录 ERROR 级别的日志到 error.log
    let error_layer = fmt::layer()
        .json()
        .with_writer(error_writer)
        .with_ansi(false)
        .with_filter(filter_fn(|metadata| {
            metadata.level() == &tracing::Level::ERROR
        }));

    // 只记录 WARN 级别的日志到 warn.log
    let warn_layer = fmt::layer()
        .json()
        .with_writer(warn_writer)
        .with_ansi(false)
        .with_filter(filter_fn(|metadata| {
            metadata.level() == &tracing::Level::WARN
        }));

    // 只记录 INFO 级别的日志到 info.log
    let info_layer = fmt::layer()
        .json()
        .with_writer(info_writer)
        .with_ansi(false)
        .with_filter(filter_fn(|metadata| {
            metadata.level() == &tracing::Level::INFO
        }));

    // 只记录 TRACE 级别的日志到 trace.log
    let trace_layer = fmt::layer()
        .json()
        .with_writer(trace_writer)
        .with_ansi(false)
        .with_filter(filter_fn(|metadata| {
            metadata.level() == &tracing::Level::TRACE
        }));

    // 标准输出记录 DEBUG 及以上级别的日志
    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG);

    tracing_subscriber::registry()
        .with(error_layer)
        .with(warn_layer)
        .with(info_layer)
        .with(trace_layer)
        .with(stdout_layer)
        .init();

    LogGuard {
        _error: error_guard,
        _warn: warn_guard,
        _info: info_guard,
        _trace: trace_guard,
    }
}
